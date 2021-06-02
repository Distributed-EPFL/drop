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

#[derive(Clone, Serialize)]
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
        Wrap{digest: self.digest.clone(), inner: self.inner.clone()}
    }
}

impl<Inner> PartialEq for Wrap<Inner>
where Inner: Serialize
{
    fn eq(&self, rho: &Wrap<Inner>) -> bool {
        self.digest == rho.digest
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