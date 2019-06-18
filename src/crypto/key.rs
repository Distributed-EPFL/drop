// Dependencies

use sodiumoxide::crypto::secretstream;
use sodiumoxide::utils;
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
