use crate::crypto::Digest;
use crate::crypto::hash;
use crate::crypto::hash::HashError;

use serde::Serialize;

use std::rc::Rc;

#[derive(Debug, Eq, Serialize)]
pub(super) struct Wrap<Inner: Serialize> {
    digest: Digest,
    #[serde(skip)] inner: Rc<Inner>
}

#[derive(Eq, Serialize)]
pub(super) enum Node<Key: Serialize, Value: Serialize> {
    Empty,
    Internal(Digest, Digest),
    Leaf(Wrap<Key>, Wrap<Value>)
}

pub(super) struct Entry<Key: Serialize, Value: Serialize> {
    pub node: Node<Key, Value>,
    pub references: usize
}

impl<Inner> Clone for Wrap<Inner>
where Inner: Serialize
{
    fn clone(&self) -> Self {
        Wrap{digest: self.digest, inner: self.inner.clone()}
    }
}

impl<Inner> PartialEq for Wrap<Inner>
where Inner: Serialize
{
    fn eq(&self, rho: &Wrap<Inner>) -> bool {
        self.digest == rho.digest
    }
}

impl<Key, Value> Clone for Node<Key, Value> 
where
    Key: Serialize,
    Value: Serialize
{
    fn clone(&self) -> Self {
        match self {
            Node::Empty => Node::Empty,
            Node::Internal(left, right) => Node::Internal(*left, *right),
            Node::Leaf(key, value) => Node::Leaf(key.clone(), value.clone())
        }
    }
}

impl<Key, Value> PartialEq for Node<Key, Value>
where
    Key: Serialize,
    Value: Serialize
{
    fn eq(&self, rho: &Node<Key, Value>) -> bool {
        match (self, rho) {
            (Node::Empty, Node::Empty) => true,
            (Node::Internal(self_left, self_right), Node::Internal(rho_left, rho_right)) => (self_left == rho_left) && (self_right == rho_right),
            (Node::Leaf(self_key, self_value), Node::Leaf(rho_key, rho_value)) => (self_key == rho_key) && (self_value == rho_value),
            _ => false
        }
    }
}

impl<Inner> Wrap<Inner>
where Inner: Serialize
{
    pub fn new(inner: Inner) -> Result<Self, HashError> {
        Ok(Wrap{digest:hash(&inner)?, inner: Rc::new(inner)})
    }

    pub fn digest(&self) -> &Digest {
        &self.digest
    }

    pub fn inner(&self) -> &Rc<Inner> {
        &self.inner
    }
}