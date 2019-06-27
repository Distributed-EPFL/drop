// Dependencies

use crate as drop;
use crate::bytewise::Load;
use crate::bytewise::Readable;
use crate::bytewise::Writable;
use sodiumoxide::crypto::secretstream;
use std::convert::Into;

// Constants

pub const SIZE: usize = 32;

// Structs

#[derive(Readable, Writable, Load, Clone)]
pub struct Key(#[bytewise] pub(super) [u8; SIZE]);

// Implemenations

impl Key {
    pub fn random() -> Self {
        secretstream::gen_key().into()
    }
}
