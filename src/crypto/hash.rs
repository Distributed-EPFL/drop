use super::errors::*;
use super::key::Key;

use bincode::serialize;

use serde::{Deserialize, Serialize};

use sodiumoxide::crypto::generichash::State as SodiumState;
use sodiumoxide::utils;

/// Static size for hashes
pub const SIZE: usize = 32;

/// Wrapper for sodium hasher
pub struct Hasher(SodiumState);

impl Hasher {
    /// Create a `Hasher` without a `Key`
    pub fn new() -> Self {
        Hasher(SodiumState::new(SIZE, None).unwrap())
    }

    /// Create a `Hasher` with a specified `Key`. <br/ >
    /// This, in effect, creates a MAC producer for authenticating data.
    pub fn keyed(key: &Key) -> Self {
        Hasher(SodiumState::new(SIZE, Some(&key.0)).unwrap())
    }

    /// Feed a chunk of bytes to this hasher
    pub fn update(&mut self, chunk: &[u8]) -> Result<(), ()> {
        self.0.update(chunk)
    }

    /// Considers the data complete and returns the resulting hash
    pub fn finalize(self) -> Result<Digest, ()> {
        self.0.finalize().map(|x| x.into())
    }
}

/// A hash digest
#[derive(Deserialize, Hash, Eq, PartialOrd, Serialize)]
pub struct Digest(pub(super) [u8; SIZE]);

impl PartialEq for Digest {
    fn eq(&self, rhs: &Digest) -> bool {
        utils::memcmp(&self.0, &rhs.0)
    }
}

fn do_hash<M: Serialize>(
    mut hasher: Hasher,
    message: &M,
) -> Result<Digest, HashError> {
    if hasher.update(serialize(message)?.as_slice()).is_err() {
        Err(SodiumError::new().into())
    } else {
        hasher.finalize().map_err(|_| SodiumError::new().into())
    }
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
            "11da6d1f761ddf9bdb4c9d6e5303ebd41f61858d0a5647a1a7bfe089bf921be9"
        );
    }

    #[test]
    fn correct_hash_string() {
        compare_digest!(
            "Hello World!",
            "aee54b3a4b95463009c1f3a1b5092c9e918fc64f9e0dd31ff7a2bf51e8614121"
        );
    }

    #[test]
    fn correct_hash_u32_array() {
        compare_digest!(
            [0u32, 1u32, 2u32, 3u32, 4u32, 5u32, 6u32, 7u32],
            "d6a648a90a8267de463f99f87849e7e7c5a9273a252e501c95b44fbb958b6f7b"
        );
    }

    #[test]
    fn correct_mac_u32() {
        compare_mac!(
            KEY,
            0u32,
            "77b158a4b3694545b41363bf4a88d5e22fb5f563e7dce933d00942fb1444070c"
        );
    }

    #[test]
    fn correct_mac_string() {
        compare_mac!(
            KEY,
            "Hello World!",
            "cd9cab2bd07de0d5e015ad1dc671b1928871b36b8961010a2d0878409133fd49"
        );
    }

    #[test]
    fn correct_mac_u32_array() {
        compare_mac!(
            KEY,
            [0u32, 1u32, 2u32, 3u32, 4u32, 5u32, 6u32, 7u32],
            "c12785392eb364193254445f8c14d8729f59713eeb0f5664eb61c9b96f4044a4"
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
