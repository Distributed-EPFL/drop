use crate::crypto::hash::{Digest, hash, SIZE as HASH_SIZE};
use crate::bytewise::Readable;
use super::errors::{SyncError, PathLengthError};

const BITS_IN_BYTE: usize = 8;

// Structs 
#[derive(Clone, Debug)]
pub struct HashPath (pub(super) Digest);

#[derive(Clone)]
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

    pub fn to_bit(&self) -> bool {
        self == &Direction::Right
    }
}

impl HashPath {
    pub const NUM_BITS: usize = HASH_SIZE * BITS_IN_BYTE;
    /// Returns the direction at a given bit index
    /// Note that this function will panic if given an index
    /// greater or equal to the number of bits in a hash digest
    pub fn at(&self, idx: usize) -> Direction {
        assert!(idx < Self::NUM_BITS, "Out of bounds on path");
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
    fn add_one(&self, dir: Direction) -> Result<PrefixedPath, PathLengthError> {
        if self.depth >= HashPath::NUM_BITS {
            return Err(PathLengthError::new())
        }

        let mut new_inner = self.inner.clone();
        let new_depth = self.depth+1;
        if self.depth % BITS_IN_BYTE == 0 {
            new_inner.push(0)
        };
        let (byte_idx, bit_idx) = split_bits(new_depth-1);
        let current_byte = new_inner.get_mut(byte_idx).unwrap();

        let new_bit = dir.to_bit();

        if new_bit == true {
            // Set the new bit
            let mask = get_mask(bit_idx);
            *current_byte = *current_byte | mask;
        } else {
            // Unset the new bit
            let mask = !get_mask(bit_idx);
            *current_byte = *current_byte & mask;
        }
        Ok(PrefixedPath{inner: new_inner, depth: new_depth})
    }

    pub fn left(&self) -> Result<PrefixedPath, PathLengthError> {
        self.add_one(Direction::Left)
    }

    pub fn right(&self) -> Result<PrefixedPath, PathLengthError> {
        self.add_one(Direction::Right)
    }

    pub fn at(&self, idx: usize) -> Option<Direction> {
        if idx < self.depth {
            let (byte_idx, bit_idx) = split_bits(idx);
            self.inner.get(byte_idx).map(|byte| Direction::from_bit(*byte, bit_idx))
        } else {
            None
        }
    }

    pub fn new(depth: usize, inner: Vec<u8>) -> Result<PrefixedPath, PathLengthError> {
        if inner.len() != (depth+BITS_IN_BYTE-1)/BITS_IN_BYTE {
            Err(PathLengthError::new())
        } else {
            Ok(PrefixedPath{depth, inner})
        }
    }

    // TODO clean up?
    pub fn is_prefix_of(&self, rhs: &HashPath) -> bool {
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

// Helper Functions

// Converts a bit index into byte index + bit-in-byte index
fn split_bits(to_split: usize) -> (usize, usize) {
    (to_split/BITS_IN_BYTE, to_split%BITS_IN_BYTE)
}

// Checks if the i-th bit is set in a byte
fn is_bit_set(byte: u8, bit_idx: usize) -> bool {
    let masked = byte & get_mask(bit_idx);
    masked != 0
}

fn get_mask(bit_idx: usize) -> u8 {
    1 << (BITS_IN_BYTE - bit_idx - 1)
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    // Direction tests

    #[test]
    fn from_bit() {
        assert_eq!(Direction::from_bit(0xFF, 0), Direction::Right);
        assert_eq!(Direction::from_bit(0, 0), Direction::Left);
    }

    #[test]
    fn to_bit() {
        assert_eq!(Direction::Right.to_bit(), true);
        assert_eq!(Direction::Left.to_bit(), false);
    }

    // Helper functions tests

    #[test]
    fn split() {
        let a = 9;
        let b = 64;
        let c = 258;
        assert_eq!(split_bits(a), (1,1));
        assert_eq!(split_bits(b), (8,0));
        assert_eq!(split_bits(c), (32, 2));
    }

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

        let byte = 0b10000100;
        for i in 0..BITS_IN_BYTE {
            if i != 0 && i != 5 {
                assert!(!is_bit_set(byte, i))
            } else {
                assert!(is_bit_set(byte, i))
            }
        }
    }

    // Full Path tests
    #[test]
    fn hashpath() {
        use Direction::*;
        // hash(15092) = 0101 1010 0001 1111
        let path = HashPath::new(&15092).unwrap();

        let expected_vec = vec!(
            Left, Right, Left, Right,
            Right, Left, Right, Left,
            Left, Left, Left, Right,
            Right, Right, Right, Right
        );

        for (idx, expected) in expected_vec.iter().enumerate() {
            assert_eq!(expected, &path.at(idx));
        }
    }

    // Prefixed tests

    #[test]
    fn prefixes() {
        let full = HashPath(Digest::try_from("0101010101000000000000000000000000000000000000000000000000000000").unwrap());

        let pref1 = PrefixedPath::new(7, vec!(0)).unwrap();
        assert!(pref1.is_prefix_of(&full), "prefix1 returned false");

        let pref2 = PrefixedPath::new(8, vec!(0b0000_0001)).unwrap();
        assert!(pref2.is_prefix_of(&full), "prefix2 returned false");

        let pref3 = PrefixedPath::new(1, vec!(0b1111_1111)).unwrap();
        assert!(!pref3.is_prefix_of(&full), "prefix3 returned true");

        let empty = PrefixedPath::new(0, Vec::new()).unwrap();
        assert!(empty.is_prefix_of(&full), "empty prefix returned false");
    }

    #[test]
    fn serialization() {
        // TODO
    }

    #[test]
    fn add_one_errors() {
        let mut inner = Vec::with_capacity(HASH_SIZE);
        for _ in 0..HASH_SIZE {
            inner.push(0);
        };
        let pref = PrefixedPath::new(HashPath::NUM_BITS, inner).unwrap();

        if let Err(e) = pref.add_one(Direction::Left) {
            println!("{:?}", e)
        } else {
            panic!("Expected an error in adding one to direction")
        }
    }

    #[test]
    fn extension() {
        let mut path = PrefixedPath::new(0, Vec::new()).unwrap();
        for i in 0..HashPath::NUM_BITS {
            assert_eq!(path.inner.len(), (i+BITS_IN_BYTE-1)/BITS_IN_BYTE);
            if i % 2 == 1 {
                path = path.left().unwrap()
            } else {
                path = path.right().unwrap()
            }
        }

        for i in 0..HASH_SIZE {
            if let Some(b) = path.inner.get(i) {
                assert_eq!(b, &0xAA)
            } else {
                panic!("Inner vector was too short")
            }
        }
    }

    #[test]
    fn prefixed_nav() {
        let inner = vec!(0xAA, 0x55);
        let inner_len = inner.len();
        let path = PrefixedPath::new(16, inner).unwrap();
        for i in 0..inner_len {
            for j in 0..BITS_IN_BYTE {
                let expected_bit = (i+j)%2 == 0;
                let actual_dir = path.at(BITS_IN_BYTE*i+j).unwrap();
                assert_eq!(actual_dir.to_bit(), expected_bit)
            }
        }
    }

    #[test]
    fn indices() {
        let prefix = PrefixedPath::new(2, vec!(0b10000000)).unwrap();
        assert_eq!(prefix.at(0), Some(Direction::Right));
        assert_eq!(prefix.at(1), Some(Direction::Left));
        assert_eq!(prefix.at(7), None);
        assert_eq!(prefix.at(64), None);
    }
}