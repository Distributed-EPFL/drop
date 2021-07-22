/// Utilities to compute a shared secret to establish a secure network stream
pub mod exchange;

use std::convert::Into;

use sodiumoxide::{
    crypto::{kx, secretstream},
    utils,
};

/// Hardcoded key size
pub const SIZE: usize = 32;

#[derive(Clone, PartialEq, Eq)]
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
