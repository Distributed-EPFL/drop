use crate as drop;
use crate::bytewise::Readable;
use crate::crypto::hash::{Digest, hash};

use super::path::*;
use super::errors::*;
use super::Syncable;

use std::mem;
use std::cell::{RefCell, Cell};

/// Private type used for the binary tree
#[derive(Debug)]
pub(super) enum Node<Data: Syncable> {

    // Empty leaf
    Empty,

    // Non-empty leaf
    Leaf {
        // Data contained in the leaf
        data: Data,
        // Potentially empty cached hash
        cached_hash: RefCell<Option<Digest>>,
    },

    Branch {
        // Pointer to the child nodes
        right: Box<Node<Data>>,
        left: Box<Node<Data>>,

        // Pre-computed values for label and size
        cached_hash: RefCell<Option<Digest>>,
        cached_size: Cell<Option<usize>>,
    }
}




impl <Data: Syncable> Node<Data> {

    // todo? add node_at_mut? (Node is a private data structure, thus all uses would
    // have to be within the syncset implementation)
    /// Finds the first node at a given path. If a (potentially empty) Leaf node is encountered
    /// prior to the path's max depth, a reference to that node is returned. 
    /// Otherwise, if the end of the path is reached, then then the iterated node will be returned
    /// by reference.
    pub fn node_at(&self, prefix: &PrefixedPath, depth: u32) -> &Node<Data> {
        if let Some(dir) = prefix.at(depth) {
            if let Node::Branch{left, right, ..} = &self {
                // Fork -> recurse into left or right
                if dir == Direction::Left {
                    left.node_at(prefix, depth+1)
                } else {
                    right.node_at(prefix, depth+1)
                }
            } else {
                // Leaf -> this is the node wanted
                &self
            }
        } else {
            // End of path reached, this is the node wanted
            &self
        }
    }

    /// Traverses the graph in a depth first manner (priority to left leaves), and applies the
    /// function to each element encountered.
    pub fn traverse<F>(&self, f: &mut F) 
    where F: FnMut(&Data) {
        use Node::*;
        match self {
            // Bottom elements
            Empty => (),
            Leaf{data,..} => f(data),

            // Recursion
            Branch{left, right, ..} => {
                left.traverse(f);
                right.traverse(f);
            },
        }
    }

    /// Returns the number of children (including itself) a node has.
    pub fn size(&self) -> usize {
        use Node::*;
        match self {
            // Empty leaf has no elements
            Empty => 0,
            
            // Branch has the sum of its left and right children's sizes
            // but a branch does not itself contain any elements
            Branch{left, right, cached_size, ..} => {

                // return cached value, or compute it and update
                if let Some(s) = cached_size.get() {
                    s
                } else {
                    let size = left.size() + right.size();
                    cached_size.replace(Some(size));
                    size
                }
            },

            // A non-empty leaf has one element
            Leaf{..} => 1
        }
    }

    /// Deletes data at the given depth, on the given path, recursively on Nodes
    pub fn delete(&mut self, data_to_delete: &Data, path: HashPath, depth: u32) -> bool {
        let deletion_successful = match self {

            // Can't delete what's not there
            Node::Empty => false,

            // Branch - recurse
            Node::Branch{ref mut left, ref mut right, ..} => {
                if path.at(depth) == Direction::Left {
                    left.delete(data_to_delete, path, depth+1)
                } else {
                    right.delete(data_to_delete, path, depth+1)
                }
            },

            // Check for potential collision, and delete if elmnt matches
            Node::Leaf{ref data,..} => {
                if data == data_to_delete {
                    true
                } else {
                    false
                }
            }
        };

        // Pull up the tree's elements
        if deletion_successful {
            // Acquire ownership and delete/clean up
            let tmp = self.swap(Node::Empty);
            let new = tmp.clean_up_node();

            // Give back ownership
            self.swap(new);
        };

        deletion_successful
    }

    // Helper function for delete()
    // Cleans up branches, and transforms leaves into Empty leaves
    // Note that this is meant to be used recursively starting at
    // the bottom
    fn clean_up_node(self) -> Node<Data> {
        use Node::*;
        match self {
            Branch{left, right, ..} => {
                match (*left, *right) {
                    // Branches with two empty leaves become an empty leaf
                    (Empty, Empty) => Empty,

                    // Branches with only one non-empty leaf pull up that leaf
                    (new @ Leaf{..}, Empty) => new,
                    (Empty, new @ Leaf{..}) => new,

                    // Everything else doesn't change, but its caches do get reset
                    (old_l, old_r) => Node::new_branch(old_l, old_r)
                }
            },

            // Leaves just become empty
            _ => Empty
        }
    }

    /// Inserts data into the node, with the given path
    pub fn insert(&mut self, data: Data, depth: u32, path: HashPath) -> Result<bool, SyncError> {
        match self {

            // Trivial case
            Node::Empty => {
                self.swap(Node::new_leaf(data));
                Ok(true)
            }
            Node::Leaf{..} => {
                let old_hash = self.hash()?;
                // Collision
                if old_hash == path.0 {
                    // Hash collision or same element inserted twice?
                    if self.cmp_data(&data) {
                        Ok(false)
                    } else {
                        Err(CollisionError::new().into())
                    }
                } else {
                    let old = self.swap(Node::Empty);
                    if let Node::Leaf{data: old_data,..} = old {
                        // Insert both elements into a new tree
                        let old_path = HashPath(old_hash);
                        let new_node = Node::make_tree(old_data, old_path, data, path, depth);

                        // No need to invalidate cache here, because we're discarding the old node anyway
                        self.swap(new_node);
                        Ok(true)
                    } else {
                        // Note: the pattern is irrefutable, but rust thinks it is refutable
                        // The reason we don't bind in the match arm is because we want to take ownership
                        // of the data, but we cannot do that because we'd be moving out of borrowed content
                        panic!("Unreachable code reached")
                    }
                }
            }
            Node::Branch{ref mut left, ref mut right, ..} => {

                // Recurse
                let success = if path.at(depth) == Direction::Left {
                    left.insert(data, depth+1, path)
                } else {
                    right.insert(data, depth+1, path)
                }?;
                // If insertion was successful, invalidate cache and propagate success up
                if success {
                    self.invalidate_cache();
                };

                Ok(success)
            }
        }
    }

    // Compare the data. Returns false for non-leaves
    fn cmp_data(&self, other: &Data) -> bool {
        match self {
            Node::Leaf{data,..} => data == other,
            _ => false
        }
    }

    // Invalidates the cache of a node
    fn invalidate_cache(&self) {
        use Node::*;
        match self {
            Empty => (),
            Leaf{cached_hash,..} => {cached_hash.replace(None);},
            Branch{cached_hash, cached_size, ..} => {
                cached_hash.replace(None);
                cached_size.replace(None);
            }
        };
    }

    // Makes a tree with 2 leaves. Do not call with path0=path1
    fn make_tree(data0: Data, path0: HashPath, data1: Data, path1: HashPath, depth: u32) -> Node<Data> {
        use Direction::*;
        debug_assert_ne!(path0, path1);
        if path0.at(depth) == Left {
            // Differing paths: exit condition
            if path1.at(depth) == Right {
                Node::new_branch_from_data(data0, path0.0, data1, path1.0)
            // Same path: recurse
            } else {
                Node::new_branch(Node::make_tree(data0, path0, data1, path1, depth+1), Node::Empty)
            }
        } else {
            // Different paths
            if path1.at(depth) == Left {
                Node::new_branch_from_data(data1, path1.0, data0, path0.0)
            // Same path
            } else {
                Node::new_branch(Node::Empty, Node::make_tree(data0, path0, data1, path1, depth+1))
            }
        }
    }

    // Convenience constructors
    fn new_leaf(data: Data) -> Node<Data> {
        Node::Leaf{data, cached_hash: RefCell::new(None)}
    }

    fn new_branch_from_data(left_data: Data, left_hash: Digest, right_data: Data, right_hash: Digest) -> Node<Data> {
        let left_node = Node::Leaf{data: left_data, cached_hash: RefCell::new(Some(left_hash))};
        let right_node = Node::Leaf{data: right_data, cached_hash: RefCell::new(Some(right_hash))};
        Node::new_branch(left_node, right_node)
    }

    // Shorthand for creating a new branch
    fn new_branch(left: Node<Data>, right: Node<Data>) -> Node<Data> {
        Node::Branch{
            left: Box::new(left),
            right: Box::new(right),
            cached_hash: RefCell::default(),
            cached_size: Cell::default()
        }
    }

    /// Mutates the node into the argument, and returns the old node
    pub fn swap(&mut self, mut new: Node<Data>) -> Node<Data> {
        mem::swap(self, &mut new);     
        new
    }

    /// Returns true for empty leaves, and false for everything else
    pub fn is_empty(&self) -> bool {
        match self {
            Node::Empty => true,
            _ => false
        }
    }

    /// Returns the node's label. This is a hash of the hashes for a branch,
    /// and the data's hash for Leaves. Empty leaves have no hash.
    pub fn hash(&self) -> Result<Digest, SyncError> {
        match self {
            // Error: hash of an empty leaf (should this be a hash of unit instead?) 
            Node::Empty => Err(EmptyHashError::new().into()),

            // Non-empty leaf - label == path == hash
            Node::Leaf{cached_hash, data} => {
                let mut cached_hash_borrowed = cached_hash.borrow_mut();
                if let Some(digest) = cached_hash_borrowed.as_ref() {
                    Ok(digest.clone())
                } else {
                    let new_hash = hash(data)?;
                    *cached_hash_borrowed = Some(new_hash.clone());
                    Ok(new_hash)
                }
            }

            Node::Branch{left, right, cached_hash,..} => {
                let mut cached_hash_borrowed = cached_hash.borrow_mut();
                if let Some(digest) = cached_hash_borrowed.as_ref() {
                    // Return cached hash
                    Ok(digest.clone())
                } else {
                    let new_hash = if left.is_empty() {
                        // Note: having two empty children to a branch would violate the invariant
                        // So we assume that !right.is_empty()
                        right.hash()?
                    } else if right.is_empty() {
                        left.hash()?
                    } else {
                        // Both elements present
                        let left_hash = left.hash()?;
                        let right_hash = right.hash()?;
                        let concat = ConcatDigest(left_hash, right_hash);
                        hash(&concat)?
                    };

                    // Update cache, return
                    *cached_hash_borrowed = Some(new_hash.clone());
                    Ok(new_hash)
                }
            }
        }
    }
}

#[derive(Readable)]
struct ConcatDigest(#[bytewise] Digest, #[bytewise] Digest);


#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::*;

    const NUM_ITERS: u32 = 50000;


    #[test]
    fn get_returns_correct_set() {

    }

    #[test]
    fn label() {
        use Node::*;
        let mut root = Empty;
        // hash(15092) = 0101 1010 0001 1111 ...
        let elem_l = 15092;
        let hash_left = hash(&elem_l).unwrap();
        // hash(13) = 1101 ...
        let elem_r = 13;
        let hash_right = hash(&elem_r).unwrap();
        root.insert(elem_l, 0, HashPath(hash_left.clone())).unwrap();
        root.insert(elem_r, 0, HashPath(hash_right.clone())).unwrap();

        let expected_label = hash(&ConcatDigest(hash_left, hash_right)).unwrap();
        assert_eq!(root.hash().unwrap(), expected_label);
    }

    #[test]
    fn traverse() {
        use Node::*;
        let mut root = Empty;
        // hash(15092) = 0101 1010 0001 1111 ...
        let elem_l = 15092;
        let hash_left = hash(&elem_l).unwrap();
        // hash(13) = 1101 ...
        let elem_r = 13;
        let hash_right = hash(&elem_r).unwrap();
        root.insert(elem_l, 0, HashPath(hash_left.clone())).unwrap();
        root.insert(elem_r, 0, HashPath(hash_right.clone())).unwrap();
        let mut total = 1;
        root.traverse(&mut |el| total*=el);
        assert_eq!(total, elem_l*elem_r, "Traversal fails for two elements");

        assert!(root.delete(&elem_l, HashPath(hash_left), 0), "Deletion fails for left element");

        total = 1;
        root.traverse(&mut |el| total*=el);
        assert_eq!(total, elem_r, "Traversal fails for one element");
    }

    #[test]
    fn insert() {
        use Node::*;
        let mut root = Empty;
        // hash(15092) = 0101 1010 0001 1111 ...
        let elem_l = 15092;
        let hash_left = hash(&elem_l).unwrap();
        // hash(13) = 1101 ...
        let elem_r = 13;
        let hash_right = hash(&elem_r).unwrap();
        root.insert(elem_l, 0, HashPath(hash_left.clone())).unwrap();
        if let Leaf{data, ..} = root {
            assert_eq!(data, elem_l, "Inserted element doesn't match");
            // Success!
        } else {
            panic!("Root is not of type Leaf. {:?}", root)
        }

        root.insert(elem_r, 0, HashPath(hash_right.clone())).unwrap();
        if let Branch{left, right, ..} = &root {
            let left: &Node<_> = left;
            let right: &Node<_> = right;
            if let (Leaf{data: data_l,..}, Leaf{data: data_r, ..}) = (left, right) {
                assert_eq!(*data_l, elem_l, "Left branch doesn't match");
                assert_eq!(*data_r, elem_r, "Right branch doesn't match");
            } else {
                panic!("Left and right branches aren't leaves, ({:?}, {:?})", left, right)
            }
        } else {
            panic!("Root is not of type Branch. {:?}", root)
        }
    }

    #[test]
    fn delete() {
        let mut root: Node<u32> = Node::Empty;
        for i in 0..NUM_ITERS {
            assert!(root.insert(i, 0, HashPath::new(&i).unwrap()).unwrap());
        }


        for i in 0..NUM_ITERS {
            let elem_path = HashPath::new(&i).unwrap();
            assert!(root.delete(&i, elem_path.clone(), 0), "Deletion fails");

            let mut nav = &root;
            for idx in 0..HashPath::NUM_BITS {
                match nav {
                    Node::Empty => break,
                    Node::Leaf{data,..} => {
                        if *data == idx as u32 {
                            panic!("Element wasn't deleted, but is supposed to have been")
                        } else {
                            break
                        }
                    },
                    Node::Branch{left, right, ..} => {
                        if left.is_empty() && right.is_empty() {
                            panic!("Dead branch encountered! Delete failed")
                        }

                        if elem_path.at(idx) == Direction::Left {
                            nav = left
                        } else {
                            nav = right
                        }
                    }
                }
            }
        }

        assert!(root.is_empty(), "Root is not empty after deleting all elements")
    }

}