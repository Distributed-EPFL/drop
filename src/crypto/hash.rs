use std::cmp;

use super::key::Key;
use super::BincodeError;

use bincode::serialize;

pub use blake3::Hash;
use blake3::Hasher as BlakeHasher;

use serde::{Deserialize, Serialize};

use snafu::{ResultExt, Snafu};

/// Static size for hashes
pub const SIZE: usize = blake3::OUT_LEN;

#[derive(Debug, Snafu)]
/// Errors enountered by [`Hasher`]
///
/// [`Hasher`]: self::Hasher
pub enum HashError {
    #[snafu(display("failed to serialize data: {}", source))]
    /// Error while serializing data for hashing
    SerializeError {
        /// Underlying error cause
        source: BincodeError,
    },
}

/// Wrapper for blake3 hasher
pub struct Hasher(BlakeHasher);

#[derive(Serialize, Deserialize)]
#[serde(remote = "Hash")]
struct SerdeDigest(#[serde(getter = "Hash::as_bytes")] pub(crate) [u8; SIZE]);

impl From<SerdeDigest> for Hash {
    fn from(d: SerdeDigest) -> Self {
        Self::from(d.0)
    }
}

/// A hash digest using blake3
#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Hash)]
pub struct Digest(#[serde(with = "SerdeDigest")] Hash);

impl Digest {
    /// Get the content of this `Digest` as a reference to a slice of bytes
    pub fn as_bytes(&self) -> &[u8; SIZE] {
        self.0.as_bytes()
    }
}

impl Ord for Digest {
    fn cmp(&self, other: &Self) -> cmp::Ordering {
        Ord::cmp(self.0.as_bytes(), other.0.as_bytes())
    }
}

impl PartialOrd for Digest {
    fn partial_cmp(&self, other: &Self) -> Option<cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<Hash> for Digest {
    fn from(h: Hash) -> Self {
        Self(h)
    }
}

impl From<[u8; SIZE]> for Digest {
    fn from(s: [u8; SIZE]) -> Self {
        Self(Hash::from(s))
    }
}

impl Hasher {
    /// Create a `Hasher` without a `Key`
    pub fn new() -> Self {
        Self(BlakeHasher::new())
    }

    /// Create a `Hasher` with a specified `Key`. <br/ >
    /// This, in effect, creates a MAC producer for authenticating data.
    pub fn keyed(key: &Key) -> Self {
        Self(BlakeHasher::new_keyed(key.as_ref()))
    }

    /// Feed a chunk of bytes to this hasher
    pub fn update(&mut self, chunk: &[u8]) {
        self.0.update(chunk);
    }

    /// Considers the data complete and returns the resulting hash
    pub fn finalize(self) -> Digest {
        self.0.finalize().into()
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

fn do_hash<M: Serialize>(
    mut hasher: Hasher,
    message: &M,
) -> Result<Digest, HashError> {
    hasher.update(&serialize(message).context(SerializeError)?);

    Ok(hasher.finalize())
}

/// Computes the cryptographic hash of the specified message.
pub fn hash<M: Serialize>(message: &M) -> Result<Digest, HashError> {
    let hasher = Hasher::new();

    do_hash(hasher, message)
}

/// Computes the message authentication code for the given message and `Key`.
pub fn authenticate<Message: Serialize>(
    key: &Key,
    message: &Message,
) -> Result<Digest, HashError> {
    let hasher = Hasher::keyed(key);

    do_hash(hasher, message)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::convert::TryFrom;
    use std::convert::TryInto;

    use super::*;

    const KEY: &str =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    macro_rules! compare_digest {
        ($data:expr, $expected:expr) => {
            let expected =
                Digest::try_from($expected).expect("failed to create digest");
            let actual = hash(&($data)).expect("failed to hash data");

            assert_eq!(actual, expected, "incorrect digest computed");
        };
    }

    macro_rules! compare_mac {
        ($key:expr, $data:expr, $expected:expr) => {
            let actual_digest = authenticate(
                &($key).try_into().expect("failed to create key"),
                &($data),
            )
            .expect("failed to hash data");
            let expected_digest: Digest =
                $expected.try_into().expect("failed to create digest");

            assert_eq!(expected_digest, actual_digest, "wrong mac computed");
        };
    }

    #[test]
    fn correct_hash_0u32() {
        compare_digest!(
            0u32,
            "ec2bd03bf86b935fa34d71ad7ebb049f1f10f87d343e521511d8f9e6625620cd"
        );
    }

    #[test]
    fn correct_hash_string() {
        compare_digest!(
            "Hello World!",
            "f081b531fffc1e69b73f40a2b08e705bd5b8dfb2396f5bc90f81118ce1286f90"
        );
    }

    #[test]
    fn correct_hash_u32_array() {
        compare_digest!(
            [0u32, 1u32, 2u32, 3u32, 4u32, 5u32, 6u32, 7u32],
            "5c7b5564a4f3fb20589ebb6d46f85373980382cdf55f71bd4955e03eb8cc2c96"
        );
    }

    #[test]
    fn correct_mac_u32() {
        compare_mac!(
            KEY,
            0u32,
            "40cc83b9a432048488badd27670dbb6fdf890ac5662b92a53f9363dbd54e023f"
        );
    }

    #[test]
    fn correct_mac_string() {
        compare_mac!(
            KEY,
            "Hello World!",
            "95302a0102fef2d232c196b8c0f585e3c6fc3b2d8da9bb4a199e8e409f893231"
        );
    }

    #[test]
    fn correct_mac_u32_array() {
        compare_mac!(
            KEY,
            [0u32, 1u32, 2u32, 3u32, 4u32, 5u32, 6u32, 7u32],
            "62f6e2709f98a117e0ae2c078eba3fbe62ba89d7aca1540c8d341bdde1d9264d"
        );
    }

    #[test]
    fn hash_collisions() {
        let mut set = HashSet::new();
        for value in 0u32..1024u32 {
            set.insert(hash(&value).unwrap());
        }

        assert_eq!(set.len(), 1024, "collisions detected");
    }

    #[test]
    fn mac_collisions() {
        let mut set = HashSet::new();
        for value in 0u32..1024u32 {
            set.insert(authenticate(&"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".try_into().unwrap(), &value).unwrap());
        }
        assert_eq!(set.len(), 1024, "collisions detected");
    }
}
