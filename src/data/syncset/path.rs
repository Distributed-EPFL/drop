use crate::crypto::hash::{Digest, hash, SIZE as HASH_SIZE};
use crate::bytewise::Readable;
use crate::bytewise::Writable;
use crate::bytewise::Writer;
use crate::bytewise::Reader;
use crate::bytewise::Load;
use crate::bytewise::Size;
use crate::bytewise::ReadError;
use crate::bytewise::WriteError;
use crate::data::Varint;
use super::errors::{SyncError, PathLengthError};

use std::convert::TryInto;

const BITS_IN_BYTE: u32 = 8;

// Structs 

/// Navigator wrapper for Digest
/// Guaranteed to have HASH_SIZE * 8 bits of depth
#[derive(Clone, Debug, PartialEq)]
pub struct HashPath (pub(super) Digest);


/// Navigator
/// Guaranteed to have 0 <= n <= HASH_SIZE * 8 bits of depth
#[derive(Clone, Debug)]
pub struct PrefixedPath {
    inner: Vec<u8>,
    depth: Varint 
}

/// Direction enumeration for abstraction of bit navigation.
/// 0 is Left, 1 is Right
#[derive(Eq, PartialEq, Debug)]
pub enum Direction {
    Left,
    Right,
}

// Implementations

impl Direction {
    /// Convert the i-th bit of the byte into a Direction
    pub fn from_bit(byte: u8, bit_idx: u32) -> Direction {
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

impl HashPath {

    /// The number of bits in a hash digest
    pub const NUM_BITS: u32 = HASH_SIZE as u32 * BITS_IN_BYTE;

    /// Returns the direction at a given bit index
    /// Note that this function will panic if given an index
    /// greater or equal to the number of bits in a hash digest
    pub fn at(&self, idx: u32) -> Direction {
        assert!(idx < Self::NUM_BITS, "Out of bounds on HashPath");
        let (byte_idx, bit_idx) = split_bits(idx);

        debug_assert!(HASH_SIZE > byte_idx as usize, "Out of bounds byte index");
        let byte = (self.0).0[byte_idx as usize];

        Direction::from_bit(byte, bit_idx)
    }

    /// Takes the i-th first bits of the digest and turn them into a PrefixedPath
    pub fn prefix(&self, depth: u32) -> PrefixedPath {
        PrefixedPath::from_digest(&self.0, depth)
    }

    /// Standard constructor
    pub fn new<Data: Readable>(data: &Data) -> Result<HashPath, SyncError> {
        let digest = hash(data)?;
        Ok(HashPath(digest))
    }
}

impl PartialEq for PrefixedPath {
    fn eq(&self, other: &PrefixedPath) -> bool {
        if self.depth == other.depth {
           let (num_full_bytes, overflow_bits) = split_bits(self.depth.0);
           let num_full_bytes = num_full_bytes.try_into().expect("Couldn't cast 32-bit integer to usize.");

            debug_assert!(self.inner.len() <= HASH_SIZE);
            debug_assert!(other.inner.len() <= HASH_SIZE);
            // overflow_bits = 0 -> num_full_bytes == inner.len
            debug_assert!(overflow_bits > 0 || num_full_bytes == self.inner.len());
            // overflow_bits > 0 -> num_full_bytes + 1 == inner.len
            debug_assert!(overflow_bits == 0 || num_full_bytes + 1 == self.inner.len());

            // Check all full bytes for equality
            for i in 0..num_full_bytes {
                unsafe {
                    if self.inner.get_unchecked(i) != other.inner.get_unchecked(i) {
                        return false
                    }
                }
            }

            // Check all the additional bits for equality
            if overflow_bits > 0 {
                let last_byte_self = unsafe{ self.inner.get_unchecked(num_full_bytes)};
                let last_byte_other = unsafe{ other.inner.get_unchecked(num_full_bytes)};
                let shift_amount = BITS_IN_BYTE - overflow_bits;

                let masked_self = last_byte_self >> shift_amount;
                let masked_other = last_byte_other >> shift_amount;
                if masked_other != masked_self {
                    return false
                }
            }

            true
        } else {
            false
        }
    }
}

impl PrefixedPath {

    fn add_one(&self, dir: Direction) -> Result<PrefixedPath, PathLengthError> {
        if self.depth.0 >= HashPath::NUM_BITS as u32 {
            return Err(PathLengthError::new())
        }

        // Copy old path, and increase depth
        let mut new_inner = self.inner.clone();
        let new_depth = self.depth.0+1;
        if self.depth.0 % BITS_IN_BYTE == 0 {
            new_inner.push(0)
        };

        // Prepare to modify last bit of new 
        let (byte_idx, bit_idx) = split_bits(new_depth-1);
        let current_byte = new_inner.get_mut(byte_idx as usize).unwrap();

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
        Ok(PrefixedPath{inner: new_inner, depth: Varint(new_depth)})
    }

    /// Extends the path with a 0
    pub fn left(&self) -> Result<PrefixedPath, PathLengthError> {
        self.add_one(Direction::Left)
    }

    /// Extends the path with a 1
    pub fn right(&self) -> Result<PrefixedPath, PathLengthError> {
        self.add_one(Direction::Right)
    }

    pub fn at(&self, idx: u32) -> Option<Direction> {
        if idx < self.depth.0 {
            let (byte_idx, bit_idx) = split_bits(idx);
            self.inner.get(byte_idx as usize)
                .map(|byte| Direction::from_bit(*byte, bit_idx))
        } else {
            None
        }
    }

    /// Creates a PrefixedPath of depth 0
    pub fn empty() -> PrefixedPath {
        PrefixedPath{inner: Vec::new(), depth: Varint(0)}
    }

    fn from_digest(digest: &Digest, depth: u32) -> PrefixedPath {
        let needed_bytes = (depth + BITS_IN_BYTE - 1) / BITS_IN_BYTE;
        let mut inner = Vec::with_capacity(needed_bytes as usize);
        for i in 0..needed_bytes {
            inner.push(digest.0[i as usize])
        };
        PrefixedPath{inner, depth: Varint(depth)}
 
    }

    /// Hashes a data element, and creates a path out of the digest
    pub fn new<Data: Readable>(data: &Data, depth: u32) -> Result<PrefixedPath, SyncError> {
        let digest = hash(data)?;
        Ok(PrefixedPath::from_digest(&digest, depth))
   }

    /// Checks if the PrefixedPath is the prefix of the full path
    pub fn is_prefix_of(&self, rhs: &HashPath) -> bool {
        let (num_full_bytes, overflow_bits) = split_bits(self.depth.0);
        let num_full_bytes = num_full_bytes.try_into().expect("Couldn't cast 32-bit integer to usize.");

        debug_assert!(self.inner.len() <= HASH_SIZE);
        // overflow_bits = 0 -> num_full_bytes == inner.len
        debug_assert!(overflow_bits > 0 || num_full_bytes == self.inner.len());
        // overflow_bits > 0 -> num_full_bytes + 1 == inner.len
        debug_assert!(overflow_bits == 0 || num_full_bytes + 1 == self.inner.len());

        if self.inner[0..num_full_bytes] != (rhs.0).0[0..num_full_bytes] {
            return false;
        }

        // If there are some bits left to individually compare in the last byte
        if overflow_bits > 0 {
            let last_byte_left = unsafe{self.inner.get_unchecked(num_full_bytes)};
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

// Trait Implementations

impl Readable for PrefixedPath {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        self.depth.accept(visitor)?;
        visitor.push(self.inner.as_slice())?;
        Ok(())
    }
}

impl Writable for PrefixedPath {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let depth = Varint::load(visitor)?;
        let num_bytes = bytes_needed(depth.0);
        let inner: Vec<u8> = visitor.pop(num_bytes)?.into();
        self.depth = depth;
        self.inner = inner;
        Ok(())
    }
}

impl Load for PrefixedPath {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        let mut res = PrefixedPath::empty();
        Writable::accept(&mut res, from)?;
        Ok(res)
    }
}

// Helper Functions

// Converts a bit index into byte index + bit-in-byte index
fn split_bits(to_split: u32) -> (u32, u32) {
    (to_split/BITS_IN_BYTE, to_split%BITS_IN_BYTE)
}

// Checks if the i-th bit is set in a byte
fn is_bit_set(byte: u8, bit_idx: u32) -> bool {
    let masked = byte & get_mask(bit_idx);
    masked != 0
}

fn get_mask(bit_idx: u32) -> u8 {
    1 << (BITS_IN_BYTE - bit_idx as u32 - 1)
}

fn bytes_needed(depth: u32) -> usize {
    ((depth + BITS_IN_BYTE - 1) / BITS_IN_BYTE) as usize
}


#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
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

    #[test]
    fn bytes_needed_test() {
        assert_eq!(bytes_needed(3), 1);
        assert_eq!(bytes_needed(0), 0);
        assert_eq!(bytes_needed(32), 4);
        assert_eq!(bytes_needed(1), 1);
    }

    // Full Path tests
    #[test]
    fn hashpath() {
        use Direction::*;
        // hash(15092) = 0101 1010 0001 1111 ...
        let path = HashPath::new(&15092).unwrap();

        let expected_vec = vec!(
            Left, Right, Left, Right,
            Right, Left, Right, Left,
            Left, Left, Left, Right,
            Right, Right, Right, Right
        );

        for (idx, expected) in expected_vec.iter().enumerate() {
            assert_eq!(expected, &path.at(idx as u32));
        }
    }

    #[test]
    #[should_panic(expected = "Out of bounds on HashPath")]
    fn depth_overflow() {
        let full = HashPath::new(&15092).unwrap();
        full.at(HashPath::NUM_BITS);
    }

    // Prefixed tests

    #[test]
    fn extension_prefixes() {
        let mut prefix = PrefixedPath::empty();
        let full = HashPath::new(&15092).unwrap();
        for depth in 0..HashPath::NUM_BITS {

            assert!(prefix.is_prefix_of(&full), "Prefix isn't prefix of full path");

            if full.at(depth) == Direction::Left {
                prefix = prefix.left().unwrap();
            } else {
                prefix = prefix.right().unwrap();
            }

            assert_eq!(depth+1, prefix.depth.0, "Prefix has wrong depth");
            for i in 0..=depth {
                assert_eq!(full.at(i),prefix.at(i).unwrap(), "Prefix doesn't match full path");
            }
        }
        assert!(prefix.is_prefix_of(&full), "Prefix isn't prefix of full path");
    }

    #[test]
    fn prefixes() {
        let full = HashPath(Digest::try_from("0101010101000000000000000000000000000000000000000000000000000000").unwrap());

        let pref1 = PrefixedPath{inner: vec!(0), depth: Varint(7)};
        assert!(pref1.is_prefix_of(&full), "prefix1 returned false");

        let pref2 = PrefixedPath{inner: vec!(0b0000_0001), depth: Varint(8)};
        assert!(pref2.is_prefix_of(&full), "prefix2 returned false");

        let pref3 = PrefixedPath{inner: vec!(0b1111_1111), depth: Varint(1)};
        assert!(!pref3.is_prefix_of(&full), "prefix3 returned true");

        let pref4 = PrefixedPath{inner: vec!(0b1111_1111, 0x01), depth: Varint(9)};
        assert!(!pref4.is_prefix_of(&full), "prefix4 returned true");

        let empty = PrefixedPath{inner: Vec::new(), depth: Varint(0)};
        assert!(empty.is_prefix_of(&full), "empty prefix returned false");
    }

    #[test]
    fn serialization() {
        use crate::bytewise::{serialize, deserialize};
        for depth in 0..=HashPath::NUM_BITS {

            let prefix = PrefixedPath::new(&15092, depth).unwrap();
            let serialized = serialize(&prefix).unwrap();


            let expected_depth_size: usize = min_bytes_to_represent(depth);

            let expected_vec_len = bytes_needed(depth);
            let expected_vec_size = expected_vec_len as usize;
            assert_eq!(serialized.len(), expected_depth_size + expected_vec_size, "Serialized version too long");


            let deserialized: PrefixedPath = deserialize(serialized.as_slice()).unwrap();

            assert_eq!(deserialized.depth, prefix.depth, "Depths didn't match");
            for i in 0..depth {
                assert_eq!(deserialized.at(i), prefix.at(i), "Deserialized didn't match original");
            }

        }
    }

    fn min_bytes_to_represent(elem: u32) -> usize {
        if elem >= 16384 {
            4
        } else if elem >= 128 {
            2
        } else {
            1
        }
    }

    #[test]
    fn add_one_errors() {
        let pref = PrefixedPath::new(&15092, HashPath::NUM_BITS).unwrap();

        if let Ok(_) = pref.add_one(Direction::Left) {
            panic!("Expected an error in adding one to direction")
        } 
    }

    #[test]
    fn prefixed_nav() {
        let inner = vec!(0xAA, 0x55);
        let inner_len = inner.len() as u32;
        let path = PrefixedPath{inner: inner, depth: Varint(16)};
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
        let prefix = PrefixedPath{inner: vec!(0b10000000), depth: Varint(2)};
        assert_eq!(prefix.at(0), Some(Direction::Right));
        assert_eq!(prefix.at(1), Some(Direction::Left));
        assert_eq!(prefix.at(7), None);
        assert_eq!(prefix.at(64), None);
    }
}