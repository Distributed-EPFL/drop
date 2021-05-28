use crate::crypto::Digest;

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex};

use super::node::Node;
use super::table::MerkleTable;

pub(super) struct Entry<Key, Value> {
    node: Node<Key, Value>,
    references: usize
}

pub(super) struct Store<Key, Value> {
    entries: HashMap<Digest, Entry<Key, Value>>
}

pub struct MerkleDatabase<Key, Value> {
    store: Arc<Mutex<Store<Key, Value>>>
}

impl<Key, Value> Clone for MerkleDatabase<Key, Value> {
    fn clone(&self) -> Self {
        MerkleDatabase{store: self.store.clone()}
    }
}

impl <Key, Value> MerkleDatabase<Key, Value> {
    pub fn new() -> Self {
        MerkleDatabase{store: Arc::new(Mutex::new(Store{entries: HashMap::new()}))}
    }

    pub fn empty_table(&self) -> MerkleTable<Key, Value> {
        MerkleTable::new(&self)
    }
}