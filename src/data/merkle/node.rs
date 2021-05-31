use crate::crypto::Digest;

use serde::Serialize;

#[derive(Serialize)]
pub(super) enum Node<Key: Serialize, Value: Serialize> {
    Empty,
    Internal(Digest, Digest),
    Leaf(Key, Value)
}