// Dependencies

use sodiumoxide::crypto::secretstream;
use std::convert::Into;

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
