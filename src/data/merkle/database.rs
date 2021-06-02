use crate::crypto::Digest;
use crate::crypto::hash;

use serde::Serialize;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use super::entry::{Entry, Node};
use super::table::MerkleTable;

pub(super) struct Constants {
    empty: Digest
}

pub(super) struct Store<Key: Serialize, Value: Serialize> {
    pub constants: Constants,
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

impl Constants {
    pub fn new<Key, Value>() -> Constants
    where
        Key: Serialize,
        Value: Serialize
    {
        Constants{empty: hash(&Node::<Key, Value>::Empty).unwrap()}
    }

    pub fn empty(&self) -> &Digest {
        &self.empty
    }
}

impl <Key, Value> MerkleDatabase<Key, Value> 
where
    Key: Serialize,
    Value: Serialize
{
    pub fn new() -> Self {
        MerkleDatabase{store: Arc::new(Mutex::new(Store{constants: Constants::new::<Key, Value>(), entries: HashMap::new()}))}
    }

    pub fn empty_table(&self) -> MerkleTable<Key, Value> {
        MerkleTable::new(&self)
    }
}