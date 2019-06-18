// Dependencies

use sodiumoxide::crypto::secretstream;
use sodiumoxide::crypto::secretstream::Key as StreamKey;
use sodiumoxide::crypto::kx::SessionKey as KxKey;
use sodiumoxide::utils;
use std::convert::From;
use std::convert::Into;
use std::ops::Drop;

// Constants

pub const SIZE: usize = 32;

// Structs

pub struct Key(pub(super) [u8; SIZE]);

// Implemenations

impl Key {
    pub fn random() -> Self {
        secretstream::gen_key().into()
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        utils::memzero(&mut self.0);
    }
}

impl From<StreamKey> for Key {
    fn from(key: StreamKey) -> Self {
        Key(key.0)
    }
}

impl From<KxKey> for Key {
    fn from(key: KxKey) -> Self {
        Key(key.0)
    }
}

impl Into<StreamKey> for Key {
    fn into(self) -> StreamKey {
        StreamKey(self.0)
    }
}

impl Into<KxKey> for Key {
    fn into(self) -> KxKey {
        KxKey(self.0)
    }
}
