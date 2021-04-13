use std::convert::TryInto;

use super::hash::Digest;
use super::key::Key;

use sodiumoxide::crypto::generichash::Digest as SodiumDigest;
use sodiumoxide::crypto::kx::SessionKey as KxKey;
use sodiumoxide::crypto::secretstream::Key as StreamKey;

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

impl From<Key> for StreamKey {
    fn from(v: Key) -> Self {
        Self(v.0)
    }
}

impl From<Key> for KxKey {
    fn from(key: Key) -> Self {
        Self(key.0)
    }
}
