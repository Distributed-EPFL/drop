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

    pub fn get(&self, prefix: &PrefixedPath, dump: bool) -> Result<Set<Data>, SyncError> {
        use Node::*;
        let node_at_prefix = self.root.node_at(prefix, 0);
        match node_at_prefix {
            Leaf{..} => {
                // Because this is a leaf, its hash is that of its data element, thus label = path
                let leaf_path = HashPath(node_at_prefix.hash()?);
                if prefix.is_prefix_of(&leaf_path) {
                    Ok(Set::new_dataset(prefix.clone(), node_at_prefix, dump))
                } else {
                    Ok(Set::new_empty_dataset(prefix.clone(), dump))
                }
            }
            Branch{..} => {
                if dump || node_at_prefix.size() <= super::DUMP_THRESHOLD {
                    Ok(Set::new_dataset(prefix.clone(), node_at_prefix, dump))
                } else {
                    Ok(Set::LabelSet{label: node_at_prefix.hash()?, path: prefix.clone()})
                }
            }
            Empty => Ok(Set::new_empty_dataset(prefix.clone(), dump)),
        }
    }

    pub fn contains(&self, data: &Data) -> Result<bool, SyncError> {
        use Node::*;
        let path = PrefixedPath::new(data, HashPath::NUM_BITS)?;
        let node_at_path = self.root.node_at(&path, 0);
        match node_at_path {
            Leaf{data: leaf_data, ..} => {
                Ok(data == leaf_data)
            }
            Empty => Ok(false),
            Branch{..} => panic!("Branch at maximum depth!")
        }
    }

    pub fn new() -> SyncSet<Data> {
        SyncSet{root: Node::Empty}
    }

    pub fn size(&self) -> usize {
        self.root.size()
    }

    pub fn start_sync(&self) -> Result<Round<Data>, SyncError> {
        let root_view = self.get(&PrefixedPath::empty(), false)?;
        Ok(Round{view: vec!(root_view), add: Vec::new(), remove: Vec::new()})
    }

    pub fn sync(&self, view: &Vec<Set<Data>>) -> Result<Round<Data>, SyncError> {
        let mut new_view: Vec<Set<Data>> = Vec::new();
        let mut to_add: Vec<Data> = Vec::new();
        let mut to_remove: Vec<Data> = Vec::new();
        for set in view {
            match set {
                Set::LabelSet{label: remote_label, path: remote_prefix} => {
                    let local_set = self.get(remote_prefix, false)?;
                    match &local_set {
                        Set::LabelSet{label: local_label,..} => {
                            if remote_label != local_label {
                                // Note: a node at max depth having children would violate invariant
                                // thus, calling unwrap is appropriate
                                new_view.push(self.get(&remote_prefix.left().unwrap(), false)?);
                                new_view.push(self.get(&remote_prefix.right().unwrap(), false)?);
                            }
                        },
                        Set::DataSet{..} => {
                            new_view.push(local_set)
                        }
                    }
                }
                Set::DataSet{underlying: remote_data, prefix: remote_prefix, dump: remote_dump} => {
                    let local_set = self.get(remote_prefix, true)?;
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

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {

    use super::*;
    extern crate rand;
    use rand::Rng;

    use std::collections::HashSet;

    const NUM_ITERS: u32 = 50000;
    #[test]
    fn get_returns_in_order() {
        let mut set = SyncSet::new();

        for i in 0..NUM_ITERS {
            set.insert(i).unwrap();
        }

        assert_eq!(set.root.size(), NUM_ITERS as usize, "Root has wrong size");

        if let Set::DataSet{underlying, ..} = set.get(&PrefixedPath::empty(), true).unwrap() {
            let mut previous = hash(underlying.get(0).expect("get() returns no elements")).unwrap();
            for i in 1..NUM_ITERS {
                let current = hash(underlying.get(i as usize).unwrap()).expect("get() returns too few elements");
                assert!(previous < current);
                previous = current;
            }
        } else {
            panic!("get() returns a LabelSet")
        }
    }

    #[test]
    fn get() {
        let mut syncset = SyncSet::new();
        let arbitrary_elem = rand::random::<u32>()%NUM_ITERS;
        for i in 0..NUM_ITERS {
            syncset.insert(i).unwrap();
        }

        let arbitrary_elem_path = HashPath::new(&arbitrary_elem).unwrap();

        for depth in 0..HashPath::NUM_BITS {
            let prefix = arbitrary_elem_path.prefix(depth);
            let set = syncset.get(&prefix, false).unwrap();
            match &set {
                Set::LabelSet{path, label} => {
                    assert!(path.is_prefix_of(&arbitrary_elem_path));
                    assert_eq!(path, &prefix, "Returned path does not match prefix");
                    if let n @ Node::Branch{..} = syncset.root.node_at(&prefix, 0) {
                        assert_eq!(&n.hash().unwrap(), label);
                    } else {
                        panic!("get returns a labelset of a leaf or empty, {:?}", set)
                    }
                },
                Set::DataSet{underlying, prefix: actual_prefix, dump} => {
                    assert!(underlying.len() <= super::super::DUMP_THRESHOLD, "Number of elements received exceeds the threshold");
                    assert!(actual_prefix.is_prefix_of(&arbitrary_elem_path), "Prefix isn't a prefix of the full hash");
                    assert_eq!(&prefix, actual_prefix, "Prefix doesn't match expected");
                    assert!(!dump, "get returns wrong value for dump")
                }
            }
        }
    }

    #[test]
    fn inserting_twice_returns_false() {
        let mut syncset: SyncSet<u64> = SyncSet::new();
        let elem = 13;
        assert!(syncset.insert(elem).unwrap(), "First insertion failed");
        assert!(!syncset.insert(elem).unwrap(), "Second insertion succeeded");
    }

    #[test]
    fn add_find() {
        let mut expected_size = 0;
        let mut set = HashSet::new();
        let mut syncset = SyncSet::new();
        let mut generator = rand::thread_rng();
        for i in 0..NUM_ITERS {
            if generator.gen() {
                expected_size+=1;
                set.insert(i);
                syncset.insert(i).unwrap();
            }
        }

        assert_eq!(syncset.size(), expected_size, "syncset has wrong size");
        for i in 0..2*NUM_ITERS {
            let should_find = set.contains(&i);
            let found = syncset.contains(&i).unwrap();
            assert_eq!(should_find, found, "Element {} present in only one of the sets", i);
        }
    }

    #[test]
    fn remove_find() {
        let mut expected_size = NUM_ITERS as usize;
        let mut set = HashSet::new();
        let mut syncset = SyncSet::new();
        let mut generator = rand::thread_rng();
        for i in 0..NUM_ITERS {
            set.insert(i);
            syncset.insert(i).unwrap();
        }

        for i in 0..NUM_ITERS {
            if generator.gen() {
                set.remove(&i);
                syncset.delete(&i).unwrap();
                expected_size-=1;
            }
        }

        assert_eq!(syncset.size(), expected_size, "Syncset has wrong size");

        for i in 0..2*NUM_ITERS {
            let should_find = set.contains(&i);
            let found = set.contains(&i);
            assert_eq!(should_find, found, "Element {} present in only one of the sets", i);
        }
    }

}