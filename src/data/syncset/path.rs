use crate::crypto::hash::{Digest, hash, SIZE as HASH_SIZE};
use crate::bytewise::Readable;
use super::syncerror::SyncError;

const BITS_IN_BYTE: usize = 8;
const NUM_BITS: usize = HASH_SIZE * BITS_IN_BYTE;

// Structs 
pub struct HashPath (pub(super) Digest);
pub struct PrefixedPath {
    inner: Vec<u8>,
    depth: usize
}

#[derive(Eq, PartialEq)]
pub(super) enum Direction {
    Left,
    Right,
}

// Implementations

impl Direction {
    pub fn from_bit(byte: u8, bit_idx: usize) -> Direction {
        if is_bit_set(byte, bit_idx) {
            Direction::Right
        } else {
            Direction::Left
        }
    }
}

impl HashPath {
    pub(super) fn at(&self, idx: usize) -> Direction {
        debug_assert!(idx < NUM_BITS, "Out of bounds on path");
        let (byte_idx, bit_idx) = split_bits(idx);
        let byte = (self.0).0[byte_idx];

        if is_bit_set(byte, bit_idx) {
            Direction::Right
        } else {
            Direction::Left
        }
    }

    pub(super) fn new<Data: Readable>(data: &Data) -> Result<HashPath, SyncError> {
        let digest = hash(data)?;
        Ok(HashPath(digest))
    }
}

impl PrefixedPath {
    pub(super) fn at(&self, idx: usize) -> Option<Direction> {
        let (byte_idx, bit_idx) = split_bits(idx);
        self.inner.get(byte_idx).map(|byte| Direction::from_bit(*byte, bit_idx))
    }
}

// Helper Functions

// Converts a given index to base 2^8
fn split_bits(to_split: usize) -> (usize, usize) {
    (to_split/BITS_IN_BYTE, to_split%BITS_IN_BYTE)
}

// Checks if the i-th bit is set in a byte
fn is_bit_set(byte: u8, bit_idx: usize) -> bool {
    let mask = 1 << bit_idx;
    let masked = byte & mask;
    masked != 0
}