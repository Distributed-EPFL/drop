use std::convert::Into;

use serde::{Deserialize, Serialize};
use sodiumoxide::crypto::secretstream;

/// Hardcoded key size
pub const SIZE: usize = 32;

#[derive(Serialize, Deserialize, Clone)]
pub struct Key(pub(super) [u8; SIZE]);

impl Key {
    pub fn random() -> Self {
        secretstream::gen_key().into()
    }
}
