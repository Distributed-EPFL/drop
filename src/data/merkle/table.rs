use crate::crypto::Digest;
use crate::crypto::hash;

use serde::Serialize;

use super::database::MerkleDatabase;
use super::entry::Node;

pub struct MerkleTable<Key: Serialize, Value: Serialize> {
    database: MerkleDatabase<Key, Value>,
    root: Digest
}

impl<Key, Value> MerkleTable<Key, Value> 
where
    Key: Serialize,
    Value: Serialize
{
    pub(super) fn new(database: &MerkleDatabase<Key, Value>) -> Self {
        MerkleTable{database: database.clone(), root: hash(&Node::<Key, Value>::Empty).unwrap()}
    }
}