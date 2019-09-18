use crate::bytewise::Readable;
use crate::crypto::hash::{Digest, hash, SIZE as HASH_SIZE};
use std::cell::RefCell;

pub struct SyncSet<Data: Readable> {
    root: Node<Data>,
}

impl <Data: Readable> SyncSet<Data> {
    pub fn insert(&mut self, data: Data) {

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

enum Direction {
    Left,
    Right,
}

impl Path {
    const BITS_IN_BYTE: usize = 8;
    const NUM_BITS: usize = HASH_SIZE * Self::BITS_IN_BYTE;
    fn at(&self, idx: usize) -> Direction {
        if idx >= Self::NUM_BITS {
            panic!("Out of bounds")
        };
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
    fn insert(&mut self, data: Data, depth: usize, path: Path) {

    }
}