use crate as drop;
use crate::bytewise::Readable;
use crate::crypto::hash::{Digest, hash, SIZE as HASH_SIZE};
use std::cell::RefCell;
use std::mem;

pub mod syncerror;
use syncerror::*;

pub struct SyncSet<Data: Readable> {
    root: Node<Data>,
}

impl <Data: Readable> SyncSet<Data> {
    pub fn insert(&mut self, data: Data) -> Result<(), SyncError> {
        let path = Path(hash(&data)?);
        self.root.insert(data, 0, path)
    }
}

enum Node<Data: Readable> {
    Empty,
    Leaf {
        data: Data,
        cached_hash: RefCell<Option<Digest>>,
    },

    Branch {
        right: Box<Node<Data>>,
        left: Box<Node<Data>>,
        cached_hash: RefCell<Option<Digest>>,
    }
}

struct Path (Digest);

#[derive(Eq, PartialEq)]
enum Direction {
    Left,
    Right,
}

impl Path {
    const BITS_IN_BYTE: usize = 8;
    const NUM_BITS: usize = HASH_SIZE * Self::BITS_IN_BYTE;
    fn at(&self, idx: usize) -> Direction {
        debug_assert!(idx < Self::NUM_BITS, "Out of bounds on path");
        let byte_idx = idx/Self::BITS_IN_BYTE;
        let bit_idx = idx%Self::BITS_IN_BYTE;
        let byte = (self.0).0[byte_idx];
        let mask = 1 << bit_idx;
        let masked = byte & mask;
        if masked == 0 {
            Direction::Left
        } else {
            Direction::Right
        }
    }
}

impl <Data: Readable> Node<Data> {
    // Inserts data into the node
    fn insert(&mut self, data: Data, depth: usize, path: Path) -> Result<(), SyncError> {
        match self {
            Node::Empty => {
                self.swap(Node::new_leaf(data));
                Ok(())
            }
            Node::Leaf{..} => {
                let old_hash = self.hash()?;
                // Collision
                if old_hash == path.0 {
                    Err(CollisionError::new().into())
                } else {
                    let old = self.swap(Node::Empty);
                    if let Node::Leaf{data: old_data,..} = old {
                        let old_path = Path(old_hash);
                        let new_node = Node::make_tree(old_data, old_path, data, path, depth);
                        self.swap(new_node);
                        Ok(())
                    } else {
                        // Note: the pattern is irrefutable, but rust thinks it is refutable
                        // The reason we don't bind in the match arm is because we want to take ownership
                        // of the data, but we cannot do that because we'd be moving out of borrowed content
                        panic!("Unreachable code reached")
                    }
                }
            }
            Node::Branch{ref mut left, ref mut right, ..} => {
                if path.at(depth) == Direction::Left {
                    left.insert(data, depth+1, path)
                } else {
                    right.insert(data, depth+1, path)
                }
            }
        }
    }

    // Makes a tree with 2 leaves. Do not call with path0=path1
    fn make_tree(data0: Data, path0: Path, data1: Data, path1: Path, depth: usize) -> Node<Data> {
        use Direction::*;
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
        Node::Branch{left: Box::new(left), right: Box::new(right), cached_hash: RefCell::new(None)}
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

    fn hash(&self) -> Result<Digest, SyncError> {
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
            Node::Branch{left, right, cached_hash} => {
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
struct ConcatDigest(#[bytewise] Digest, #[bytewise] Digest);