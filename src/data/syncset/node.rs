use crate as drop;
use crate::bytewise::Readable;
use crate::crypto::hash::{Digest, hash};

use super::path::*;
use super::errors::*;
use super::Set;
use super::{DUMP_THRESHOLD, Syncable};

use std::mem;
use std::cell::{RefCell, Cell};

#[derive(Debug)]
pub(super) enum Node<Data: Syncable> {
    Empty,
    Leaf {
        data: Data,
        cached_hash: RefCell<Option<Digest>>,
    },

    Branch {
        right: Box<Node<Data>>,
        left: Box<Node<Data>>,
        cached_hash: RefCell<Option<Digest>>,
        cached_size: Cell<Option<usize>>,
    }
}




impl <Data: Syncable> Node<Data> {

    pub fn node_at(&self, prefix: PrefixedPath, depth: u32) -> &Node<Data> {
        if let Some(dir) = prefix.at(depth) {
            if let Node::Branch{left, right, ..} = &self {
                if dir == Direction::Left {
                    left.node_at(prefix, depth+1)
                } else {
                    right.node_at(prefix, depth+1)
                }
            } else {
                &self
            }
        } else {
            &self
        }
    }

    pub fn get(&self, prefix: PrefixedPath, depth: u32, dump: bool) -> Result<Set<Data>, SyncError> {
        use Node::*;
        // todo: refactor to use node_at
        if let Some(dir) = prefix.at(depth) {
            match self {
                Empty => Ok(Set::new_empty_dataset(prefix, dump)),
                Branch{left, right, ..} => {
                    if dir == Direction::Left {
                        left.get(prefix, depth+1, dump)
                    } else {
                        right.get(prefix, depth+1, dump)
                    }
                },
                Leaf{..} => {
                    let self_path = HashPath(self.hash()?);
                    if prefix.is_prefix_of(&self_path) {
                        Ok(Set::new_dataset(prefix, &self, dump))
                    } else {
                        Ok(Set::new_empty_dataset(prefix, dump))
                    }
                }
            }
        } else {
            match self {
                Branch{..} => {
                        if dump || self.size() < DUMP_THRESHOLD {
                        Ok(Set::new_dataset(prefix, self, dump))
                    } else {
                        Ok(Set::LabelSet{label: self.hash()?, path: prefix})
                    }
                }
                Leaf{..} => {
                    Ok(Set::new_dataset(prefix, self, dump))
                }
                Empty => {
                    Ok(Set::new_empty_dataset(prefix, dump))
                }
            }
        }
    }

    pub fn traverse<F>(&self, f: &mut F) 
    where F: FnMut(&Data) {
        use Node::*;
        match self {
            Empty => (),
            Branch{left, right, ..} => {
                left.traverse(f);
                right.traverse(f);
            },
            Leaf{data,..} => f(data)
        }
    }

    pub fn size(&self) -> usize {
        use Node::*;
        match self {
            Empty => 0,
            Branch{left, right, cached_size, ..} => {
                if let Some(s) = cached_size.get() {
                    s
                } else {
                    left.size() + right.size()
                }
            },
            Leaf{..} => 1
        }
    }

    pub fn delete(&mut self, data_to_delete: &Data, path: HashPath, depth: u32) -> bool {
        let deletion_successful = match self {
            Node::Empty => false,
            Node::Branch{ref mut left, ref mut right, ..} => {
                if path.at(depth) == Direction::Left {
                    left.delete(data_to_delete, path, depth+1)
                } else {
                    right.delete(data_to_delete, path, depth+1)
                }
            },
            Node::Leaf{ref data,..} => {
                if data == data_to_delete {
                    true
                } else {
                    false
                }
            }
        };

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
    fn clean_up_node(self) -> Node<Data> {
        use Node::*;
        match self {
            Branch{left, right, ..} => {
                match (*left, *right) {
                    (Empty, Empty) => Empty,
                    (new @ Leaf{..}, Empty) => new,
                    (Empty, new @ Leaf{..}) => new,
                    (old_l, old_r) => Node::new_branch(old_l, old_r)
                }
            },
            _ => Empty
        }
    }

    // Inserts data into the node
    pub fn insert(&mut self, data: Data, depth: u32, path: HashPath) -> Result<bool, SyncError> {
        match self {
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

    fn swap(&mut self, mut new: Node<Data>) -> Node<Data> {
        mem::swap(self, &mut new);     
        new
    }

    fn is_empty(&self) -> bool {
        match self {
            Node::Empty => true,
            _ => false
        }
    }

    pub fn hash(&self) -> Result<Digest, SyncError> {
        match self {
            Node::Empty => Err(EmptyHashError::new().into()),
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
                    Ok(digest.clone())
                } else {
                    let new_hash = if left.is_empty() {
                        // Note: having two empty children to a branch would violate the invariant
                        right.hash()?
                    } else if right.is_empty() {
                        left.hash()?
                    } else {
                        let left_hash = left.hash()?;
                        let right_hash = right.hash()?;
                        let concat = ConcatDigest(left_hash, right_hash);
                        hash(&concat)?
                    };
                    *cached_hash_borrowed = Some(new_hash.clone());
                    Ok(new_hash)
                }
            }
        }
    }
}

#[derive(Readable)]
pub(super) struct ConcatDigest(#[bytewise] Digest, #[bytewise] Digest);


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