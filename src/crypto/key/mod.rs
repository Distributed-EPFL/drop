/// Utilities to compute a shared secret to establish a secure network stream
pub mod exchange;

use std::convert::Into;

use serde::{Deserialize, Serialize};

use sodiumoxide::crypto::kx;
use sodiumoxide::crypto::secretstream;
use sodiumoxide::utils;

/// Hardcoded key size
pub const SIZE: usize = 32;

#[derive(Clone, Deserialize, Eq, PartialOrd, Ord, Serialize)]
/// A symmetric cryptographic `Key`
pub struct Key([u8; SIZE]);

impl Key {
    /// Generate a new random `Key`
    pub fn random() -> Self {
        secretstream::gen_key().into()
    }
}

impl AsRef<[u8; SIZE]> for Key {
    fn as_ref(&self) -> &[u8; SIZE] {
        &self.0
    }
}

impl From<[u8; SIZE]> for Key {
    fn from(s: [u8; SIZE]) -> Self {
        Self(s)
    }
}

impl From<secretstream::Key> for Key {
    fn from(key: secretstream::Key) -> Self {
        Key(key.0)
    }
}

impl From<kx::SessionKey> for Key {
    fn from(key: kx::SessionKey) -> Self {
        Key(key.0)
    }
}

impl From<Key> for secretstream::Key {
    fn from(v: Key) -> Self {
        Self(v.0)
    }
}

impl From<Key> for kx::SessionKey {
    fn from(key: Key) -> Self {
        Self(key.0)
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        utils::memzero(&mut self.0);
    }
}

impl PartialEq for Key {
    fn eq(&self, rhs: &Key) -> bool {
        utils::memcmp(&self.0, &rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;

    use sodiumoxide::utils::increment_le;

    #[test]
    fn symmetric_key_ordering() {
        let key = Key::random();
        let mut key_plus_one = key.clone();

        increment_le(&mut key_plus_one.0);

        assert_eq!(
            key.cmp(&key_plus_one),
            Ordering::Less,
            "failed to compare keys correctly"
        );
    }
}
