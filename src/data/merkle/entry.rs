use crate::crypto::Digest;

use serde::Serialize;

use std::rc::Rc;

#[derive(Serialize)]
pub(super) enum Node<Key: Serialize, Value: Serialize> {
    Empty,
    Internal(Digest, Digest),
    Leaf(Key, Rc<Value>)
}

pub(super) struct Entry<Key: Serialize, Value: Serialize> {
    pub node: Node<Key, Value>,
    pub references: usize
}