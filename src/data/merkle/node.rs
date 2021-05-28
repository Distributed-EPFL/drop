use crate::crypto::Digest;

pub(super) enum Node<Key, Value> {
    Empty,
    Internal(Digest, Digest),
    Leaf(Key, Value)
}