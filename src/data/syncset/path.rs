use std::convert::TryInto;

use crate::crypto::HashError;
use crate::crypto::hash::{hash, Digest, SIZE as HASH_SIZE};
use super::errors::PathLengthError;

use serde::{Serialize, Deserialize};

const BITS_IN_BYTE: usize = 8;

/// Navigator wrapper for Digest
/// Guaranteed to have HASH_SIZE * 8 bits of depth
#[derive(Clone, Debug, PartialEq)]
pub struct Path(pub(super) Digest);

/// Navigator
/// Guaranteed to have 0 <= n <= HASH_SIZE * 8 bits of depth
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Prefix {
    inner: [u8; HASH_SIZE],
    depth: usize,
}

/// Direction enumeration for abstraction of bit navigation.
/// 0 is Left, 1 is Right
#[derive(Eq, PartialEq, Debug)]
pub enum Direction {
    Left,
    Right,
}

impl Direction {
    /// Convert the i-th bit of the byte into a Direction
    pub fn from_bit(byte: u8, bit_idx: usize) -> Direction {
        if is_bit_set(byte, bit_idx) {
            Direction::Right
        } else {
            Direction::Left
        }
    }

    /// Convert the Direction to a bit
    pub fn to_bit(&self) -> bool {
        self == &Direction::Right
    }
}

impl Path {
    /// The number of bits in a hash digest
    pub const NUM_BITS: usize = HASH_SIZE * BITS_IN_BYTE;

    /// Returns the direction at a given bit index
    /// Note that this function will panic if given an index
    /// greater or equal to the number of bits in a hash digest
    pub fn at(&self, idx: usize) -> Result<Direction, PathLengthError> {
        if idx < Path::NUM_BITS {
            let (byte_idx, bit_idx) = split_bits(idx);

            let byte = (self.0).0[byte_idx as usize];

            Ok(Direction::from_bit(byte, bit_idx))
        } else {
            Err(PathLengthError::new("Out of bounds on path"))
        }
    }

    /// Takes the i-th first bits of the digest and turn them into a Prefix
    pub fn prefix(&self, depth: usize) -> Prefix {
        Prefix::from_digest(&self.0, depth)
    }

    /// Standard constructor
    pub fn new<Data: Serialize>(data: &Data) -> Result<Path, HashError> {
        let digest = hash(data)?;
        Ok(Path(digest))
    }
}

impl PartialEq for Prefix {
    fn eq(&self, other: &Prefix) -> bool {
        if self.depth == other.depth {
            let (num_full_bytes, overflow_bits) = split_bits(self.depth);
            let num_full_bytes = num_full_bytes
                .try_into()
                .expect("Couldn't cast 32-bit integer to usize.");

            // Check all full bytes for equality
            for i in 0..num_full_bytes {
                unsafe {
                    if self.inner.get_unchecked(i) != other.inner.get_unchecked(i) {
                        return false;
                    }
                }
            }

            // Check all the additional bits for equality
            if overflow_bits > 0 {
                let last_byte_self = unsafe { self.inner.get_unchecked(num_full_bytes) };
                let last_byte_other = unsafe { other.inner.get_unchecked(num_full_bytes) };
                let shift_amount = BITS_IN_BYTE - overflow_bits;

                let masked_self = last_byte_self >> shift_amount;
                let masked_other = last_byte_other >> shift_amount;
                if masked_other != masked_self {
                    return false;
                }
            }

            true
        } else {
            false
        }
    }
}

impl Prefix {
    fn add_one(&self, dir: Direction) -> Result<Prefix, PathLengthError> {
        if self.depth >= Path::NUM_BITS {
            return Err(PathLengthError::new("Cannot add depth to max-depth Prefix"));
        }

        // Copy old path, and increase depth
        let mut new_inner = self.inner;
        let new_depth = self.depth + 1;

        // Prepare to modify last bit of new
        let (byte_idx, bit_idx) = split_bits(new_depth - 1);
        let current_byte = new_inner.get_mut(byte_idx as usize).unwrap();

        let new_bit = dir.to_bit();

        if new_bit {
            // Set the new bit
            let mask = get_mask(bit_idx);
            *current_byte |= mask;
        } else {
            // Unset the new bit
            let mask = !get_mask(bit_idx);
            *current_byte &= mask;
        }
        Ok(Prefix {
            inner: new_inner,
            depth: new_depth,
        })
    }

    /// Extends the path with a 0
    pub fn left(&self) -> Result<Prefix, PathLengthError> {
        self.add_one(Direction::Left)
    }

    /// Extends the path with a 1
    pub fn right(&self) -> Result<Prefix, PathLengthError> {
        self.add_one(Direction::Right)
    }

    pub fn at(&self, idx: usize) -> Option<Direction> {
        if idx < self.depth {
            let (byte_idx, bit_idx) = split_bits(idx);
            let byte = self.inner[byte_idx as usize];
            let dir = Direction::from_bit(byte, bit_idx);
            Some(dir)
        } else {
            None
        }
    }

    /// Creates a Prefix of depth 0
    pub fn empty() -> Prefix {
        Prefix {
            inner: [0; HASH_SIZE],
            depth: 0,
        }
    }

    fn from_digest(digest: &Digest, depth: usize) -> Prefix {
        Prefix {
            inner: digest.0,
            depth,
        }
    }

    /// Hashes a data element, and creates a path out of the digest
    pub fn new<Data: Serialize>(data: &Data, depth: usize) -> Result<Prefix, HashError> {
        let digest = hash(data)?;
        Ok(Prefix::from_digest(&digest, depth))
    }

    /// Checks if the Prefix is the prefix of the full path
    pub fn is_prefix_of(&self, rhs: &Path) -> bool {
        let (num_full_bytes, overflow_bits) = split_bits(self.depth);
        let num_full_bytes = num_full_bytes
            .try_into()
            .expect("Couldn't cast 32-bit integer to usize.");

        if self.inner[0..num_full_bytes] != (rhs.0).0[0..num_full_bytes] {
            return false;
        }

        // If there are some bits left to individually compare in the last byte
        if overflow_bits > 0 {
            let last_byte_left = unsafe { self.inner.get_unchecked(num_full_bytes) };
            let last_byte_right = (rhs.0).0[num_full_bytes];
            let shift_amount = BITS_IN_BYTE - overflow_bits;

            // Right shift to truncate irrelevant bits
            let left_masked = last_byte_left >> shift_amount;
            let right_masked = last_byte_right >> shift_amount;

            if left_masked != right_masked {
                return false;
            }
        }
        true
    }
}


// Converts a bit index into byte index + bit-in-byte index
fn split_bits(to_split: usize) -> (usize, usize) {
    (to_split / BITS_IN_BYTE, to_split % BITS_IN_BYTE)
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
    use std::convert::TryFrom;

    use super::*;

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
        assert_eq!(split_bits(a), (1, 1));
        assert_eq!(split_bits(b), (8, 0));
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
            let b = i % 2 == 0;
            assert_eq!(is_bit_set(mishmash, i), b);
        }

        let byte = 0b1000_0100;
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
        // hash(15092) = 0101 1010 0001 1111 ...
        let path = Path::new(&15092).unwrap();

        let expected_vec = vec![
            Left, Right, Left, Right, Right, Left, Right, Left, Left, Left, Left, Right, Right,
            Right, Right, Right,
        ];

        for (idx, expected) in expected_vec.iter().enumerate() {
            assert_eq!(expected, &path.at(idx).unwrap());
        }
    }

    #[test]
    fn depth_overflow() {
        let full = Path::new(&15092).unwrap();
        if full.at(Path::NUM_BITS).is_ok() {
            panic!("Path returns Ok at max depth")
        }
    }

    // Prefixed tests

    #[test]
    fn extension_prefixes() {
        let mut prefix = Prefix::empty();
        let full = Path::new(&15092).unwrap();
        for depth in 0..Path::NUM_BITS {
            assert!(
                prefix.is_prefix_of(&full),
                "Prefix isn't prefix of full path"
            );

            if full.at(depth).unwrap() == Direction::Left {
                prefix = prefix.left().unwrap();
            } else {
                prefix = prefix.right().unwrap();
            }

            assert_eq!(depth + 1, prefix.depth, "Prefix has wrong depth");
            for i in 0..=depth {
                assert_eq!(
                    full.at(i).unwrap(),
                    prefix.at(i).unwrap(),
                    "Prefix doesn't match full path"
                );
            }
        }
        assert!(
            prefix.is_prefix_of(&full),
            "Prefix isn't prefix of full path"
        );
    }

    #[test]
    fn prefixes() {
        let full = Path(
            Digest::try_from("0101010101000000000000000000000000000000000000000000000000000000")
                .unwrap(),
        );

        let pref1 = Prefix {
            inner: [0; HASH_SIZE],
            depth: 7,
        };
        assert!(pref1.is_prefix_of(&full), "prefix1 returned false");

        let pref2 = Prefix {
            inner: [0x01; HASH_SIZE],
            depth: 8,
        };
        assert!(pref2.is_prefix_of(&full), "prefix2 returned false");

        let pref3 = Prefix {
            inner: [0xFF; HASH_SIZE],
            depth: 1,
        };
        assert!(!pref3.is_prefix_of(&full), "prefix3 returned true");

        let pref4 = Prefix {
            inner: [0xFF, 0x1, 0, 0, 0, 0, 0, 0,
                       0,   0, 0, 0, 0, 0, 0, 0,
                       0,   0, 0, 0, 0, 0, 0, 0,
                       0,   0, 0, 0, 0, 0, 0, 0],
            depth: 9,
        };
        assert!(!pref4.is_prefix_of(&full), "prefix4 returned true");

        let empty = Prefix {
            inner: [0;HASH_SIZE],
            depth: 0,
        };
        assert!(empty.is_prefix_of(&full), "empty prefix returned false");
    }

    #[test]
    fn add_one_errors() {
        let pref = Prefix::new(&15092, Path::NUM_BITS).unwrap();

        if pref.add_one(Direction::Left).is_ok() {
            panic!("Expected an error in adding one to direction")
        }
    }

    #[test]
    fn prefix_nav() {
        let mut inner = [0; HASH_SIZE];
        inner[0] = 0xAA;
        inner[1] = 0x55;
        let inner_len = 2;
        let path = Prefix {
            inner,
            depth: 16,
        };
        for i in 0..inner_len {
            for j in 0..BITS_IN_BYTE {
                let expected_bit = (i + j) % 2 == 0;
                let actual_dir = path.at(BITS_IN_BYTE * i + j).unwrap();
                assert_eq!(actual_dir.to_bit(), expected_bit)
            }
        }
    }

    #[test]
    fn indices() {
        let prefix = Prefix {
            inner: [0b1000_0000;HASH_SIZE],
            depth: 2,
        };
        assert_eq!(prefix.at(0), Some(Direction::Right));
        assert_eq!(prefix.at(1), Some(Direction::Left));
        assert_eq!(prefix.at(7), None);
        assert_eq!(prefix.at(64), None);
    }
}
