// Dependencies

use sodiumoxide::utils;
use super::hash::Digest;
use super::key::Key;

// Implementations

impl PartialEq<Digest> for Digest {
    fn eq(&self, rhs: &Digest) -> bool {
        utils::memcmp(&self.0, &rhs.0)
    }
}

impl Eq for Digest {}

impl PartialEq<Key> for Key {
    fn eq(&self, rhs: &Key) -> bool {
        utils::memcmp(&self.0, &rhs.0)
    }
}

impl Eq for Key {}
