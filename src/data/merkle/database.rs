use crate::crypto::Digest;

use serde::Serialize;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::node::Node;
use super::table::MerkleTable;

pub(super) struct Entry<Key: Serialize, Value: Serialize> {
    pub node: Node<Key, Value>,
    pub references: usize
}

pub(super) struct Store<Key: Serialize, Value: Serialize> {
    pub entries: HashMap<Digest, Entry<Key, Value>>
}

pub struct MerkleDatabase<Key: Serialize, Value: Serialize> {
    pub(super) store: Arc<Mutex<Store<Key, Value>>>
}

impl<Key, Value> Clone for MerkleDatabase<Key, Value> 
where 
    Key: Serialize, 
    Value: Serialize 
{
    fn clone(&self) -> Self {
        MerkleDatabase{store: self.store.clone()}
    }
}

impl <Key, Value> MerkleDatabase<Key, Value> 
where
    Key: Serialize,
    Value: Serialize
{
    pub fn new() -> Self {
        MerkleDatabase{store: Arc::new(Mutex::new(Store{entries: HashMap::new()}))}
    }

    pub fn empty_table(&self) -> MerkleTable<Key, Value> {
        MerkleTable::new(&self)
    }
}