use crate::crypto::hash::{Digest, hash, SIZE as HASH_SIZE};
use crate::bytewise::Readable;
use super::syncerror::SyncError;

pub(super) struct Path (pub(super) Digest);

#[derive(Eq, PartialEq)]
pub(super) enum Direction {
    Left,
    Right,
}

impl Path {
    const BITS_IN_BYTE: usize = 8;
    const NUM_BITS: usize = HASH_SIZE * Self::BITS_IN_BYTE;
    pub(super) fn at(&self, idx: usize) -> Direction {
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

    pub(super) fn new<Data: Readable>(data: &Data) -> Result<Path, SyncError> {
        let digest = hash(data)?;
        Ok(Path(digest))
    }
}