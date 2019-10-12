// Dependencies
use super::path::*;
use super::errors::*;
use super::{Node, Set};
use super::Syncable;
use crate::crypto::hash::hash;

// Syncset
pub struct SyncSet<Data: Syncable> {
    root: Node<Data>,
}

// Round, the structure used to sync Syncsets
pub struct Round<Data: Syncable> {
    pub view: Vec<Set<Data>>,
    pub add: Vec<Data>,
    pub remove: Vec<Data>
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


    pub fn start_sync(&self) -> Result<Round<Data>, SyncError> {
        let root_view = self.get(PrefixedPath::new(0, vec!()).unwrap(), false)?;
        Ok(Round{view: vec!(root_view), add: Vec::new(), remove: Vec::new()})
    }

    pub fn sync(&self, view: &Vec<Set<Data>>) -> Result<Round<Data>, SyncError> {
        let mut new_view: Vec<Set<Data>> = Vec::new();
        let mut to_add: Vec<Data> = Vec::new();
        let mut to_remove: Vec<Data> = Vec::new();
        for set in view {
            match set {
                Set::LabelSet{label: remote_label, prefix: remote_prefix} => {
                    let local_set = self.get(remote_prefix.clone(), false)?;
                    match &local_set {
                        Set::LabelSet{label: local_label,..} => {
                            if remote_label != local_label {
                                // Note: a node at max depth having children would violate invariant
                                // thus, calling unwrap is appropriate
                                new_view.push(self.get(remote_prefix.left().unwrap(), false)?);
                                new_view.push(self.get(remote_prefix.right().unwrap(), false)?);
                            }
                        },
                        Set::DataSet{..} => {
                            new_view.push(local_set)
                        }
                    }
                }
                Set::DataSet{underlying: remote_data, prefix: remote_prefix, dump: remote_dump} => {
                    let local_set = self.get(remote_prefix.clone(), true)?;
                    if let Set::DataSet{underlying: local_data, ..} = &local_set {
                        if remote_data != local_data {
                            let mut local_hash_opt = None;
                            let mut remote_hash_opt = None;
                            let mut i = 0;
                            let mut j = 0;
                            // Since the data is ordered we can do a merge like in a merge-sort
                            while i < remote_data.len() && j < local_data.len() {
                                // Update hashes
                                if local_hash_opt == None {
                                    local_hash_opt = Some(hash(unsafe {
                                        local_data.get_unchecked(j)
                                    })?);
                                };

                                if remote_hash_opt == None {
                                    remote_hash_opt = Some(hash(unsafe {
                                        remote_data.get_unchecked(i)
                                    })?);
                                };
                                
                                // Borrow, explicitely avoid moving out
                                let local_hash = local_hash_opt.as_ref().unwrap();
                                let remote_hash = remote_hash_opt.as_ref().unwrap();

                                if remote_hash < local_hash {
                                    let new = unsafe {
                                        remote_data.get_unchecked(i)
                                        }.clone();
                                    to_add.push(new);
                                    i+=1;
                                    remote_hash_opt = None;
                                } else if remote_hash > local_hash {
                                    let new = unsafe {
                                        local_data.get_unchecked(i)
                                    }.clone();

                                    to_remove.push(new);
                                    j+=1;
                                    local_hash_opt = None;
                                } else {
                                    i+=1;
                                    j+=1;
                                    remote_hash_opt = None;
                                    local_hash_opt = None;
                                }
                            }

                            while i < remote_data.len() {
                                to_add.push(unsafe{remote_data.get_unchecked(i)}.clone());
                                i+=1;
                            }

                            while j < local_data.len() {
                                to_remove.push(unsafe{local_data.get_unchecked(j)}.clone());
                                j+=1;
                            }

                            // Since the remote syncset wasn't dumping, this means that its set was small enough to send over the network
                            // Meaning we should give it our entire set at the prefix
                            if !remote_dump {
                                new_view.push(local_set);
                            }
                        }
                    }

                }
            }
        }
        Ok(Round{add: to_add, remove: to_remove, view: new_view})
    }
}