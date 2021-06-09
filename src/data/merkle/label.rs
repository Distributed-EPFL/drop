use crate::crypto::hash::{Digest, SIZE};
use crate::crypto::hash::hash;

use serde::Serialize;

use super::entry::Node;

#[derive(Clone, Copy, Eq, PartialEq, Serialize)]
pub(super) enum Label {
    Filled(Digest),
    Empty
}

impl From<Digest> for Label {
    fn from(digest: Digest) -> Self {
        Label::Filled(digest)
    }
}

impl Label {
    pub fn is_filled(&self) -> bool {
        *self != Label::Empty
    }

    pub fn is_empty(&self) -> bool {
        *self == Label::Empty
    }

    pub fn as_digest(&self) -> &Digest {
        match self {
            Label::Filled(digest) => digest,
            Label::Empty => panic!("called `Label::as_digest()` on an `Empty` value")
        }
    }
}

pub(super) fn label<Key, Value>(node: &Node<Key, Value>) -> Label 
where
    Key: Serialize,
    Value: Serialize
{
    match node {
        Node::Empty => Label::Empty,
        node => hash(&node).unwrap().into()
    }
}