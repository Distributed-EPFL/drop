// Dependencies

use sodiumoxide::utils;
use std::ops::Drop;
use super::hash::Digest;
use super::key::Key;

// Implementations

impl PartialEq<Digest> for Digest {
    fn eq(&self, rhs: &Digest) -> bool {
        utils::memcmp(&self.0, &rhs.0)
    }
}

impl Eq for Digest {}

impl Drop for Key {
    fn drop(&mut self) {
        utils::memzero(&mut self.0);
    }
}

impl PartialEq<Key> for Key {
    fn eq(&self, rhs: &Key) -> bool {
        utils::memcmp(&self.0, &rhs.0)
    }
}

impl Eq for Key {}
