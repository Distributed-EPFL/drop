/// Utilities to compute a shared secret to establish a secure network stream
pub mod exchange;

use crypto_secretstream as secretstream;
use rand::{rngs::OsRng, RngCore};

/// Hardcoded key size
pub const SIZE: usize = secretstream::Key::BYTES;

#[derive(Clone, PartialEq, Eq)]
/// A symmetric cryptographic `Key`
pub struct Key([u8; SIZE]);

impl Key {
    /// Generate a new random `Key`
    pub fn random() -> Self {
        let mut bytes = [0u8; SIZE];
        OsRng.fill_bytes(&mut bytes);

        Self(bytes)
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
        let mut bytes = [0u8; SIZE];
        bytes.clone_from_slice(key.as_ref());

        Self(bytes)
    }
}

impl From<Key> for secretstream::Key {
    fn from(v: Key) -> Self {
        Self::from(v.0)
    }
}
