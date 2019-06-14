// Dependencies

use crate::bytewise::ReadError;
use crate::bytewise::Readable;
use crate::bytewise::Reader;
use crate::bytewise::ReaderError;
use sodiumoxide::crypto::generichash::State as SodiumState;
use sodiumoxide::crypto::generichash::Digest as SodiumDigest;
use sodiumoxide::utils;
use std::default::Default;
use std::convert::From;
use std::convert::TryFrom;
use std::convert::TryInto;
use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use super::errors::MalformedHex;
use super::errors::ParseHexError;
use super::errors::UnexpectedSize;

// Constants

pub const SIZE: usize = 32;

// Structs

pub struct State(SodiumState);

#[derive(Hash, PartialOrd)]
pub struct Digest([u8; SIZE]);

// Functions

pub fn hash<Acceptor: Readable>(acceptor: &Acceptor) -> Result<Digest, ReadError> {
    let mut state = State::new();
    state.visit(acceptor)?;
    Ok(state.finalize())
}

// Implementations

impl State {
    pub fn new() -> Self {
        State(SodiumState::new(SIZE, None).unwrap())
    }

    pub fn finalize(self) -> Digest {
        self.0.finalize().unwrap().into()
    }
}

impl Reader for State {
    fn push(&mut self, chunk: &[u8]) -> Result<(), ReaderError> {
        self.0.update(chunk).unwrap();
        Ok(())
    }
}

impl From<SodiumDigest> for Digest {
    fn from(digest: SodiumDigest) -> Self {
        Digest(digest[..SIZE].try_into().unwrap())
    }
}

impl TryFrom<&str> for Digest {
    type Error = ParseHexError;

    fn try_from(hex: &str) -> Result<Self, ParseHexError> {
        if hex.len() != (2 * SIZE) { Err(UnexpectedSize::new().into()) } else {
            let mut digest: [u8; SIZE] = Default::default();
            for index in 0..SIZE {
                digest[index] = u8::from_str_radix(&hex[(2 * index)..(2 * (index + 1))], 16)
                                    .map_err(|_| ParseHexError::from(MalformedHex::new()))?;
            }

            Ok(Digest(digest))
        }
    }
}

impl PartialEq<Digest> for Digest {
    fn eq(&self, rhs: &Digest) -> bool {
        utils::memcmp(&self.0, &rhs.0)
    }
}

impl Eq for Digest {}

impl Display for Digest {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "<")?;
        for byte in &self.0 { write!(fmt, "{:02x}", byte)?; }
        write!(fmt, ">")?;

        Ok(())
    }
}

impl Debug for Digest {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self)
    }
}

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Test cases

    #[test]
    fn from() {
        assert_eq!(format!("{}", Digest::try_from("0000000000000000000000000000000000000000000000000000000000000000").unwrap()), "<0000000000000000000000000000000000000000000000000000000000000000>");
        assert_eq!(format!("{:?}", Digest::try_from("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap()), "<0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef>");

        Digest::try_from("").unwrap_err();
        Digest::try_from("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcde").unwrap_err();
        Digest::try_from("0123456789abcdef0123456789abqdef0123456789abcdef0123456789abcdef").unwrap_err();
    }

    #[test]
    fn reference() {
        assert_eq!(hash(&0u32).unwrap(), Digest::try_from("11da6d1f761ddf9bdb4c9d6e5303ebd41f61858d0a5647a1a7bfe089bf921be9").unwrap());
        assert_eq!(hash(&"Hello World!".to_string()).unwrap(), Digest::try_from("975077d5bb150ca2dafda69096aeb20eabd2010edc6f2352b389954fa485b700").unwrap());
        assert_eq!(hash(&[0u32, 1u32, 2u32, 3u32, 4u32, 5u32, 6u32, 7u32]).unwrap(), Digest::try_from("d6a648a90a8267de463f99f87849e7e7c5a9273a252e501c95b44fbb958b6f7b").unwrap());
    }

    #[test]
    fn collisions() {
        let mut set = HashSet::new();
        for value in 0u32..1024u32 { set.insert(hash(&value).unwrap()); }
        assert_eq!(set.len(), 1024);
    }
}
