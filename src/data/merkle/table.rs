use super::database::MerkleDatabase;

pub struct MerkleTable<Key, Value> {
    database: MerkleDatabase<Key, Value>
}

impl<Key, Value> MerkleTable<Key, Value> {
    pub(super) fn new(database: &MerkleDatabase<Key, Value>) -> Self {
        MerkleTable{database: database.clone()}
    }
}