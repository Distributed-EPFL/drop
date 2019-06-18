// Dependencies

use sodiumoxide::crypto::generichash::Digest as SodiumDigest;
use sodiumoxide::crypto::secretstream::Key as StreamKey;
use sodiumoxide::crypto::kx::SessionKey as KxKey;
use std::convert::TryInto;
use super::hash::Digest;
use super::key::Key;

// Implementations

impl From<SodiumDigest> for Digest {
    fn from(digest: SodiumDigest) -> Self {
        Digest(digest[..].try_into().unwrap())
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
