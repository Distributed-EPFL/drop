use std::{
    cell::{Cell, RefCell},
    mem,
};

use snafu::ResultExt;

use super::{errors::*, path::*, Syncable};
use crate::crypto::hash::{hash, Digest};

/// Private type used for the binary tree
#[derive(Debug)]
pub(super) enum Node<Data: Syncable> {
    // Empty leaf
    Empty,

    // Non-empty leaf
    Leaf {
        // Data contained in the leaf
        item: Data,
        // Potentially empty cached hash
        hash: Digest,
    },

    Internal {
        // Pointer to the child nodes
        right: Box<Node<Data>>,
        left: Box<Node<Data>>,

        // Pre-computed values for label and size
        cached_label: RefCell<Option<Digest>>,
        cached_size: Cell<Option<usize>>,
    },
}

impl<Data: Syncable> Node<Data> {
    /// Finds the first node at a given path. If a (potentially empty) Leaf node is encountered
    /// prior to the path's max depth, a reference to that node is returned.
    /// Otherwise, if the end of the path is reached, then then the iterated node will be returned
    /// by reference.
    pub fn node_at(&self, prefix: &Prefix, depth: usize) -> &Node<Data> {
        if let Some(dir) = prefix.at(depth) {
            if let Node::Internal { left, right, .. } = &self {
                // Fork -> recurse into left or right
                if dir == Direction::Left {
                    left.node_at(prefix, depth + 1)
                } else {
                    right.node_at(prefix, depth + 1)
                }
            } else {
                // Leaf -> this is the node wanted
                self
            }
        } else {
            // End of path reached, this is the node wanted
            self
        }
    }

    pub fn dump(&self) -> Vec<&Data> {
        let mut result = Vec::with_capacity(self.size());
        self.dump_recursive(&mut result);
        debug_assert_eq!(result.len(), self.size());
        result
    }

    fn dump_recursive<'a>(&'a self, result: &mut Vec<&'a Data>) {
        match self {
            Node::Leaf { item, .. } => result.push(item),
            Node::Empty => (),
            Node::Internal { left, right, .. } => {
                left.dump_recursive(result);
                right.dump_recursive(result);
            }
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
            Internal {
                left,
                right,
                cached_size,
                ..
            } => {
                // return cached value, or compute it and update
                if let Some(s) = cached_size.get() {
                    s
                } else {
                    let size = left.size() + right.size();
                    cached_size.replace(Some(size));
                    size
                }
            }

            // A non-empty leaf has one element
            Leaf { .. } => 1,
        }
    }

    /// Deletes item at the given depth, on the given path, recursively on Nodes
    pub fn delete(
        &mut self,
        item_to_delete: &Data,
        path: Path,
        depth: usize,
    ) -> bool {
        let deletion_successful = match self {
            // Can't delete what's not there
            Node::Empty => false,

            // Branch - recurse
            Node::Internal {
                ref mut left,
                ref mut right,
                ..
            } => {
                if path.at(depth).expect("Recursion at max depth happened")
                    == Direction::Left
                {
                    left.delete(item_to_delete, path, depth + 1)
                } else {
                    right.delete(item_to_delete, path, depth + 1)
                }
            }

            // Check for potential collision, and delete if elmnt matches
            Node::Leaf { ref item, .. } => item == item_to_delete,
        };

        // Pull up the tree's elements
        if deletion_successful {
            // Acquire ownership and delete/clean up
            let tmp = self.swap(Node::Empty);
            let new = tmp.pull_up_delete();

            // Give back ownership
            self.swap(new);
        };

        deletion_successful
    }

    // Helper function for delete()
    // Cleans up branches, and transforms leaves into Empty leaves
    // Note that this is meant to be used recursively starting at
    // the bottom
    fn pull_up_delete(self) -> Node<Data> {
        use Node::*;
        match self {
            Internal { left, right, .. } => {
                match (*left, *right) {
                    // Branches with two empty leaves become an empty leaf
                    (Empty, Empty) => Empty,

                    // Branches with only one non-empty leaf pull up that leaf
                    (new @ Leaf { .. }, Empty) => new,
                    (Empty, new @ Leaf { .. }) => new,

                    // Everything else doesn't change, but its caches do get reset
                    (old_l, old_r) => Node::new_internal(old_l, old_r),
                }
            }

            // Leaves just become empty
            _ => Empty,
        }
    }

    /// Inserts item into the node, with the given path
    pub fn insert(
        &mut self,
        item: Data,
        depth: usize,
        path: Path,
    ) -> Result<bool, SyncError> {
        match self {
            // Trivial case
            Node::Empty => {
                self.swap(Node::new_leaf(item, path.0));
                Ok(true)
            }
            Node::Leaf { .. } => {
                let old_hash = self.label()?;
                // Collision
                if old_hash == path.0 {
                    // Hash collision or same element inserted twice?
                    if self.cmp_item(&item) {
                        Ok(false)
                    } else {
                        Collision.fail()
                    }
                } else {
                    let old = self.swap(Node::Empty);
                    if let Node::Leaf { item: old_item, .. } = old {
                        // Insert both elements into a new tree
                        let old_path = Path(old_hash);
                        let new_node = Node::make_tree(
                            old_item, old_path, item, path, depth,
                        );

                        // No need to invalidate cache here, because we're discarding the old node anyway
                        self.swap(new_node);
                        Ok(true)
                    } else {
                        // Note: the pattern is irrefutable, but rust thinks it is refutable
                        // The reason we don't bind in the match arm is because we want to take ownership
                        // of the item, but we cannot do that because we'd be moving out of borrowed content
                        panic!("Unreachable code reached")
                    }
                }
            }
            Node::Internal {
                ref mut left,
                ref mut right,
                ..
            } => {
                // Recurse
                let success =
                    if path.at(depth).expect("Recursion at max depth happened")
                        == Direction::Left
                    {
                        left.insert(item, depth + 1, path)
                    } else {
                        right.insert(item, depth + 1, path)
                    }?;
                // If insertion was successful, invalidate cache and propagate success up
                if success {
                    self.invalidate_cache();
                };

                Ok(success)
            }
        }
    }

    // Compare the item. Returns false for non-leaves
    fn cmp_item(&self, other: &Data) -> bool {
        match self {
            Node::Leaf { item, .. } => item == other,
            _ => false,
        }
    }

    // Invalidates the cache of a node
    fn invalidate_cache(&self) {
        use Node::*;
        if let Internal {
            cached_label,
            cached_size,
            ..
        } = self
        {
            cached_label.replace(None);
            cached_size.replace(None);
        }
    }

    // Makes a tree with 2 leaves. Do not call with path0=path1
    fn make_tree(
        item0: Data,
        path0: Path,
        item1: Data,
        path1: Path,
        depth: usize,
    ) -> Node<Data> {
        use Direction::*;
        debug_assert_ne!(path0, path1);
        if path0.at(depth).expect(
            "make_tree(): tried to insert two elements at identical paths",
        ) == Left
        {
            // Differing paths: exit condition
            if path1.at(depth).expect(
                "make_tree(): tried to insert two elements at identical paths",
            ) == Right
            {
                Node::new_internal_from_items(item0, path0.0, item1, path1.0)
            // Same path: recurse
            } else {
                Node::new_internal(
                    Node::make_tree(item0, path0, item1, path1, depth + 1),
                    Node::Empty,
                )
            }
        } else {
            // Different paths
            if path1.at(depth).expect(
                "make_tree(): tried to insert two elements at identical paths",
            ) == Left
            {
                Node::new_internal_from_items(item1, path1.0, item0, path0.0)
            // Same path
            } else {
                Node::new_internal(
                    Node::Empty,
                    Node::make_tree(item0, path0, item1, path1, depth + 1),
                )
            }
        }
    }

    // Convenience constructors
    fn new_leaf(item: Data, hash: Digest) -> Node<Data> {
        Node::Leaf { item, hash }
    }

    fn new_internal_from_items(
        left_item: Data,
        left_hash: Digest,
        right_item: Data,
        right_hash: Digest,
    ) -> Node<Data> {
        let left_node = Node::Leaf {
            item: left_item,
            hash: left_hash,
        };
        let right_node = Node::Leaf {
            item: right_item,
            hash: right_hash,
        };
        Node::new_internal(left_node, right_node)
    }

    // Shorthand for creating a new branch
    fn new_internal(left: Node<Data>, right: Node<Data>) -> Node<Data> {
        Node::Internal {
            left: Box::new(left),
            right: Box::new(right),
            cached_label: RefCell::default(),
            cached_size: Cell::default(),
        }
    }

    /// Mutates the node into the argument, and returns the old node
    pub fn swap(&mut self, mut new: Node<Data>) -> Node<Data> {
        mem::swap(self, &mut new);
        new
    }

    /// Returns true for empty leaves, and false for everything else
    pub fn is_empty(&self) -> bool {
        matches!(self, Node::Empty)
    }

    /// Returns the node's label. This is a hash of the hashes for a branch,
    /// and the item's hash for Leaves. Empty leaves have no hash.
    pub fn label(&self) -> Result<Digest, SyncError> {
        match self {
            // Error: hash of an empty leaf (should this be a hash of unit instead?)
            Node::Empty => EmptyHash.fail(),

            // Non-empty leaf: label == path == hash
            Node::Leaf { hash, .. } => Ok(*hash),

            Node::Internal {
                left,
                right,
                cached_label,
                ..
            } => {
                let mut cached_label_borrowed = cached_label.borrow_mut();
                if let Some(digest) = cached_label_borrowed.as_ref() {
                    // Return cached hash
                    Ok(*digest)
                } else {
                    let new_hash = if left.is_empty() {
                        // Note: having two empty children to a branch would violate the invariant
                        // So we assume that !right.is_empty()
                        right.label()?
                    } else if right.is_empty() {
                        left.label()?
                    } else {
                        // Both elements present
                        let left_hash = left.label()?;
                        let right_hash = right.label()?;
                        let concat = ConcatDigest(left_hash, right_hash);
                        hash(&concat).context(Hash)?
                    };

                    // Update cache, return
                    *cached_label_borrowed = Some(new_hash);
                    Ok(new_hash)
                }
            }
        }
    }
}

#[derive(serde::Serialize)]
struct ConcatDigest(Digest, Digest);

#[cfg(test)]
mod tests {
    use super::*;

    const NUM_ITERS: u32 = 50000;

    #[test]
    fn get_returns_correct_set() {}

    #[test]
    fn label() {
        use Node::*;
        let mut root = Empty;

        let elem_r = 15092;
        let hash_right = hash(&elem_r).unwrap();

        // hash(13) = 1101 ...
        let elem_l = 13;
        let hash_left = hash(&elem_l).unwrap();

        root.insert(elem_l, 0, Path(hash_left)).unwrap();
        root.insert(elem_r, 0, Path(hash_right)).unwrap();

        let expected_label =
            hash(&ConcatDigest(hash_left, hash_right)).unwrap();
        assert_eq!(root.label().unwrap(), expected_label);
    }

    #[test]
    fn insert() {
        use Node::*;
        let mut root = Empty;
        // hash(15092) = 0101 1010 0001 1111 ...
        let elem_r = 15092;
        let hash_right = hash(&elem_r).unwrap();

        // hash(13) = 1101 ...
        let elem_l = 13;
        let hash_left = hash(&elem_l).unwrap();

        root.insert(elem_l, 0, Path(hash_left)).unwrap();
        if let Leaf { item, .. } = root {
            assert_eq!(item, elem_l, "Inserted element doesn't match");
        // Success!
        } else {
            panic!("Root is not of type Leaf. {:?}", root)
        }

        root.insert(elem_r, 0, Path(hash_right)).unwrap();
        if let Internal { left, right, .. } = &root {
            let left: &Node<_> = left;
            let right: &Node<_> = right;
            if let (Leaf { item: item_l, .. }, Leaf { item: item_r, .. }) =
                (left, right)
            {
                assert_eq!(*item_l, elem_l, "Left branch doesn't match");
                assert_eq!(*item_r, elem_r, "Right branch doesn't match");
            } else {
                panic!(
                    "Left and right branches aren't leaves, ({:?}, {:?})",
                    left, right
                )
            }
        } else {
            panic!("Root is not of type Branch. {:?}", root)
        }
    }

    #[test]
    fn delete() {
        let mut root: Node<u32> = Node::Empty;
        for i in 0..NUM_ITERS {
            assert!(root.insert(i, 0, Path::new(&i).unwrap()).unwrap());
        }

        for i in 0..NUM_ITERS {
            let elem_path = Path::new(&i).unwrap();
            assert!(root.delete(&i, elem_path.clone(), 0), "Deletion fails");

            let mut nav = &root;
            for idx in 0..Path::NUM_BITS {
                match nav {
                    Node::Empty => break,
                    Node::Leaf { item, .. } => {
                        if *item == idx as u32 {
                            panic!("Element wasn't deleted, but is supposed to have been")
                        } else {
                            break;
                        }
                    }
                    Node::Internal { left, right, .. } => {
                        if left.is_empty() && right.is_empty() {
                            panic!("Dead branch encountered! Delete failed")
                        }

                        if elem_path.at(idx).unwrap() == Direction::Left {
                            nav = left
                        } else {
                            nav = right
                        }
                    }
                }
            }
        }

        assert!(
            root.is_empty(),
            "Root is not empty after deleting all elements"
        )
    }
}
