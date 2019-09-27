use crate::crypto::hash::{Digest, hash, SIZE as HASH_SIZE};
use crate::bytewise::Readable;
use super::errors::{SyncError, PathLengthError};

const BITS_IN_BYTE: usize = 8;

// Structs 
#[derive(Clone)]
pub struct HashPath (pub(super) Digest);
pub struct PrefixedPath {
    inner: Vec<u8>,
    depth: usize
}

#[derive(Eq, PartialEq, Debug)]
pub enum Direction {
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
    pub const NUM_BITS: usize = HASH_SIZE * BITS_IN_BYTE;
    pub fn at(&self, idx: usize) -> Direction {
        debug_assert!(idx < Self::NUM_BITS, "Out of bounds on path");
        let (byte_idx, bit_idx) = split_bits(idx);
        let byte = (self.0).0[byte_idx];
        Direction::from_bit(byte, bit_idx)
    }

    pub fn new<Data: Readable>(data: &Data) -> Result<HashPath, SyncError> {
        let digest = hash(data)?;
        Ok(HashPath(digest))
    }
}

impl PrefixedPath {
    pub fn at(&self, idx: usize) -> Option<Direction> {
        if idx < self.depth {
            let (byte_idx, bit_idx) = split_bits(idx);
            self.inner.get(byte_idx).map(|byte| Direction::from_bit(*byte, bit_idx))
        } else {
            None
        }
    }

    pub fn new(depth: usize, inner: Vec<u8>) -> Result<PrefixedPath, PathLengthError> {
        if inner.len() < (depth+BITS_IN_BYTE-1)/BITS_IN_BYTE {
            Err(PathLengthError::new())
        } else {
            Ok(PrefixedPath{depth, inner})
        }
    }

    // TODO clean up?
    pub fn is_prefix_of(&self, rhs: &HashPath) -> bool {
       if self.depth > HashPath::NUM_BITS {
           false
       } else {
           let (num_full_bytes, overflow_bits) = split_bits(self.depth);
           for i in 0..num_full_bytes {
               if let Some(byte) = self.inner.get(i) {
                   if *byte != (rhs.0).0[i] {
                       return false;
                   }
               } else {
                   return false
               };
           }
           if overflow_bits > 0 {
               if let Some(last_byte_left) = self.inner.get(num_full_bytes) {
                   let last_byte_right = (rhs.0).0[num_full_bytes];
                   let shift_amount = overflow_bits;
                   let left_masked = last_byte_left >> shift_amount;
                   let right_masked = last_byte_right >> shift_amount;
                   if left_masked != right_masked {
                       return false;
                   }
               } else {
                   return false
               }
           }
           true
       }
    }
}

// Helper Functions

// Converts a given index to base 2^8
fn split_bits(to_split: usize) -> (usize, usize) {
    (to_split/BITS_IN_BYTE, to_split%BITS_IN_BYTE)
}

// Checks if the i-th bit is set in a byte
fn is_bit_set(byte: u8, bit_idx: usize) -> bool {
    let mask = 1 << (BITS_IN_BYTE - bit_idx - 1);
    let masked = byte & mask;
    masked != 0
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    #[test]
    fn bits() {
        let ones: u8 = 0xFF;
        let zeroes: u8 = 0x00;
        let mishmash: u8 = 0xAA;
        for i in 0..BITS_IN_BYTE {
            assert!(is_bit_set(ones, i));
            assert!(!is_bit_set(zeroes, i));
            let b = i%2==0;
            assert_eq!(is_bit_set(mishmash, i), b);
        }
    }

    #[test]
    fn prefixes() {
        let full = HashPath(Digest::try_from("0101010101000000000000000000000000000000000000000000000000000000").unwrap());
        assert_eq!((full.0).0[0], 0b00000001);

        let pref = PrefixedPath::new(7, vec!(0)).expect("Prefixed path coudn't be created");
        assert_eq!(*pref.inner.get(0).unwrap(), 0b00000000);

        assert!(pref.is_prefix_of(&full), "prefix didn't return true");

        let pref2 = PrefixedPath::new(8, vec!(1)).unwrap();
        assert_eq!(pref2.inner[0], 0b00000001);

        assert!(pref.is_prefix_of(&full), "prefix returned false");
    }

    #[test]
    fn indices() {
        let prefix = PrefixedPath::new(0, Vec::new()).unwrap();
        assert_eq!(prefix.at(0), None);
        assert_eq!(prefix.at(7), None);
        assert_eq!(prefix.at(64), None);
    }
}