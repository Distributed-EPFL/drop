pub mod exchange;

use std::convert::Into;

use serde::{Deserialize, Serialize};

use sodiumoxide::crypto::secretstream;
use sodiumoxide::utils;

/// Hardcoded key size
pub const SIZE: usize = 32;

#[derive(Clone, Deserialize, Eq, PartialOrd, Ord, Serialize)]
/// A symmetric cryptographic `Key`
pub struct Key(pub(super) [u8; SIZE]);

impl Key {
    /// Generate a new random `Key`
    pub fn random() -> Self {
        secretstream::gen_key().into()
    }
}

impl Drop for Key {
    fn drop(&mut self) {
        utils::memzero(&mut self.0);
    }
}

impl PartialEq for Key {
    fn eq(&self, rhs: &Key) -> bool {
        utils::memcmp(&self.0, &rhs.0)
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;

    use sodiumoxide::utils::increment_le;

    #[test]
    fn symmetric_key_ordering() {
        let key = Key::random();
        let mut key_plus_one = key.clone();

        increment_le(&mut key_plus_one.0);

        assert_eq!(
            key.cmp(&key_plus_one),
            Ordering::Less,
            "failed to compare keys correctly"
        );
    }
}
