use crate::crypto::hash::{Digest, SIZE};
use crate::crypto::hash::hash;

use serde::Serialize;

use super::entry::Node;

pub(super) const EMPTY: Digest = Digest([0u8; SIZE]);

pub(super) fn label<Key, Value>(node: &Node<Key, Value>) -> Digest 
where
    Key: Serialize,
    Value: Serialize
{
    match node {
        Node::Empty => EMPTY,
        node => hash(&node).unwrap()
    }
}