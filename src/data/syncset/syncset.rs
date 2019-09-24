// Dependencies
use crate::bytewise::Readable;
use super::path::*;
use super::syncerror::*;
use super::Node;

// Syncset
pub struct SyncSet<Data: Readable + PartialEq> {
    root: Node<Data>,
}

impl <Data: Readable + PartialEq> SyncSet<Data> {
    pub fn insert(&mut self, data: Data) -> Result<bool, SyncError> {
        let path = HashPath::new(&data)?;
        self.root.insert(data, 0, path)
    }

    pub fn delete(&mut self, data_to_delete: &Data) -> Result<bool, SyncError> {
        let path = HashPath::new(data_to_delete)?;
        Ok(self.root.delete(data_to_delete, path, 0))
    }
}