// Dependencies
use super::path::*;
use super::errors::*;
use super::{Node, Set};
use super::Syncable;

// Syncset
pub struct SyncSet<Data: Syncable> {
    root: Node<Data>,
}

// Syncset implementation
impl <Data: Syncable> SyncSet<Data> {
    pub fn insert(&mut self, data: Data) -> Result<bool, SyncError> {
        let path = HashPath::new(&data)?;
        self.root.insert(data, 0, path)
    }

    pub fn delete(&mut self, data_to_delete: &Data) -> Result<bool, SyncError> {
        let path = HashPath::new(data_to_delete)?;
        Ok(self.root.delete(data_to_delete, path, 0))
    }

    pub fn get(&self, prefix: PrefixedPath, dump: bool) -> Result<Set<Data>, SyncError> {
        self.root.get(prefix, 0, dump)
    }
}