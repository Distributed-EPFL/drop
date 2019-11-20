// Dependencies

use crate::bytewise::ReadError;
use crate::bytewise::Readable;
use crate::bytewise::Reader;
use crate::bytewise::ReaderError;
use sodiumoxide::crypto::generichash::State as SodiumState;
use super::key::Key;

// Constants

pub const SIZE: usize = 32;

// Structs

pub struct State(SodiumState);

#[derive(Hash, PartialOrd)]
pub struct Digest(pub(super) [u8; SIZE]);

// Functions

pub fn hash<Acceptor: Readable>(acceptor: &Acceptor) -> Result<Digest, ReadError> {
    let mut state = State::new();
    state.visit(acceptor)?;
    Ok(state.finalize())
}

pub fn authenticate<Acceptor: Readable>(key: &Key, acceptor: &Acceptor) -> Result<Digest, ReadError> {
    let mut state = State::keyed(key);
    state.visit(acceptor)?;
    Ok(state.finalize())
}

// Implementations

impl State {
    pub fn new() -> Self {
        State(SodiumState::new(SIZE, None).unwrap())
    }

    pub fn keyed(key: &Key) -> Self {
        State(SodiumState::new(SIZE, Some(&key.0)).unwrap())
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

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::convert::TryFrom;

    // Test cases

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
