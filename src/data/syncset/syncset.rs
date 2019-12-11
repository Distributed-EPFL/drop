use crate::crypto::hash::hash;
use super::Set;
use super::Syncable;
use super::errors::*;
use super::node::Node;
use super::path::*;

pub struct SyncSet<Data: Syncable> {
    root: Node<Data>,
}

// Round, the structure used to sync Syncsets
#[derive(Debug, Clone)]
pub struct Round<'a, 'b, Data: Syncable> {
    pub view: Vec<Set<&'a Data>>,
    pub add: Vec<&'b Data>,
    pub remove: Vec<&'a Data>,
}

/// A Set based on Merkle trees, with efficient (O(K log N), K number of differences,
/// N number of total items) symmetric difference computation.
/// Note that the SyncErrors returned by most of the functions here
/// can almost always only happen due to Read errors. Thus, if you're using the tree to store
/// something simple like integers, it is safe to assume the operations on this tree will
/// never return errors (ignoring edge cases like hash collisions)

impl<Data: Syncable> SyncSet<Data> {
    /// Attempts to insert the given element into the set.
    /// Returns Ok(true) if the element was successfully inserted,
    /// Ok(false) if it was already present
    /// Note that unlike all of the other functions implemented here, this can
    /// also fail when a hash collision occurs
    pub fn insert(&mut self, data: Data) -> Result<bool, SyncError> {
        let path = Path::new(&data)?;
        self.root.insert(data, 0, path)
    }

    /// Attempts to delete the given element from the set, and
    /// returns Ok(true) if the element was contained in the
    /// syncset, Ok(false) if it wasn't
    pub fn delete(&mut self, data_to_delete: &Data) -> Result<bool, SyncError> {
        let path = Path::new(data_to_delete)?;
        Ok(self.root.delete(data_to_delete, path, 0))
    }

    /// Returns the Set of nodes at the Path, the dump parameter determines
    /// if the entire sub-tree at the path should be returned, regardless of size.
    /// For instance, calling get(...) with an empty prefix, and dump set to true
    /// will return all the elements of the set.
    pub fn get(&self, prefix: &Prefix, dump: bool) -> Result<Set<&Data>, SyncError> {
        use Node::*;

        let node_at_prefix = self.root.node_at(prefix, 0);
        match node_at_prefix {
            Leaf { .. } => {
                // Because this is a leaf, its hash is that of its data element, thus label == path
                let leaf_path = Path(node_at_prefix.label()?);

                // Check if the prefix actually matches, or if we just ran out of nodes
                if prefix.is_prefix_of(&leaf_path) {
                    Ok(Set::new_dataset(prefix.clone(), node_at_prefix, dump))
                } else {
                    Ok(Set::new_empty_dataset(prefix.clone(), dump))
                }
            }
            Internal { .. } => {
                // We either dump the branch, or we return the Label/Path structure corresponding
                if dump || node_at_prefix.size() <= super::DUMP_THRESHOLD {
                    Ok(Set::new_dataset(prefix.clone(), node_at_prefix, dump))
                } else {
                    Ok(Set::LabelSet {
                        label: node_at_prefix.label()?,
                        prefix: prefix.clone(),
                    })
                }
            }

            // Empty leaf -> empty set
            Empty => Ok(Set::new_empty_dataset(prefix.clone(), dump)),
        }
    }

    /// Checks if the element is contained in the set
    pub fn contains(&self, data: &Data) -> Result<bool, SyncError> {
        use Node::*;
        let path = Prefix::new(data, Path::NUM_BITS)?;
        let node_at_path = self.root.node_at(&path, 0);
        match node_at_path {
            Leaf {
                item: leaf_data, ..
            } => Ok(data == leaf_data),
            Empty => Ok(false),
            Internal { .. } => panic!("Branch at maximum depth!"),
        }
    }

    /// Creates a new Set with an empty root
    pub fn new() -> SyncSet<Data> {
        SyncSet { root: Node::Empty }
    }

    /// Returns the number of elements contained in the set
    pub fn size(&self) -> usize {
        self.root.size()
    }

    /// Returns the inital Round
    pub fn start_sync(&self) -> Result<Round<Data>, SyncError> {
        let root_view = self.get(&Prefix::empty(), false)?;
        Ok(Round {
            view: vec![root_view],
            add: Vec::new(),
            remove: Vec::new(),
        })
    }

    /// Synchronises two sets. Please note that this is a fairly low-level function.
    /// The view argument contains all the nodes to scrutinize.
    /// Round.remove will contain the nodes that this SyncSet contains but aren't
    /// present in the view, Round.add will contain the nodes that are present in
    /// the view, but not in this set.
    pub fn sync<'a,'b>(&'a self, view: &'b Vec<Set<Data>>) -> Result<Round<'a, 'b, Data>, SyncError> {
        // Figure out how much to pre-allocate
        let mut elem_count = 0;
        for set in view {
            if let Set::ListSet { underlying, .. } = set {
                elem_count += underlying.len();
            }
        }

        let mut new_view: Vec<Set<&Data>> = Vec::with_capacity(view.len() * 2);
        let mut add: Vec<&Data> = Vec::with_capacity(elem_count);
        let mut remove: Vec<&Data> = Vec::with_capacity(elem_count);

        // Iterate over the view, and compare each set at each path to this syncset's set at the
        // same path
        for set in view {
            match set {
                // Label-path structure, check own label, and send back result
                Set::LabelSet {
                    label: remote_label,
                    prefix: remote_prefix,
                } => {
                    let local_set = self.get(remote_prefix, false)?;
                    match &local_set {
                        // Check label, and if it doesn't match, send back the two children
                        Set::LabelSet {
                            label: local_label, ..
                        } => {
                            if remote_label != local_label {
                                // Note: a node at max depth having children would violate invariant
                                // thus, calling unwrap is appropriate
                                new_view.push(self.get(&remote_prefix.left().unwrap(), false)?);
                                new_view.push(self.get(&remote_prefix.right().unwrap(), false)?);
                            }
                        }

                        // Dump threshold reached; send everything
                        Set::ListSet { .. } => new_view.push(local_set),
                    }
                }

                // Full-data set structure. Compare everything at the path
                Set::ListSet {
                    underlying: remote_data,
                    prefix: remote_prefix,
                    dump: remote_dump,
                } => {
                    let local_set = self.get(remote_prefix, true)?;

                    // This pattern is irrefutable; get() was called with dump = true
                    if let Set::ListSet {
                        underlying: local_data,
                        ..
                    } = &local_set
                    {
                        // Cannot use standard vector comparison; one vector owns its data, while the other one not
                        let mut vectors_equal = remote_data.len() == local_data.len();
                        for i in 0..remote_data.len() {
                            if !vectors_equal {
                                break;
                            }

                            unsafe {
                                vectors_equal = remote_data.get_unchecked(i) == *local_data.get_unchecked(i);
                            }
                        }

                        if !vectors_equal {
                            // Keep track of hashes to avoid recomputing them at every iteration
                            let mut local_hash_opt = None;
                            let mut remote_hash_opt = None;

                            let mut i = 0;
                            let mut j = 0;
                            // Since the data is ordered we can do a merge like in a merge-sort
                            // While we have elements in both of the arrays, we iterate
                            while i < remote_data.len() && j < local_data.len() {
                                // Update hashes
                                if local_hash_opt == None {
                                    local_hash_opt =
                                        Some(hash(unsafe { local_data.get_unchecked(j) })?);
                                };

                                if remote_hash_opt == None {
                                    remote_hash_opt =
                                        Some(hash(unsafe { remote_data.get_unchecked(i) })?);
                                };

                                // Borrow, explicitely avoid moving out
                                let local_hash = local_hash_opt.as_ref().unwrap();
                                let remote_hash = remote_hash_opt.as_ref().unwrap();

                                // Add elements in order
                                if remote_hash < local_hash {
                                    let new = unsafe { remote_data.get_unchecked(i) };
                                    add.push(new);
                                    i += 1;
                                    remote_hash_opt = None;
                                } else if remote_hash > local_hash {
                                    let new = unsafe { local_data.get_unchecked(i) }.clone();

                                    remove.push(new);
                                    j += 1;
                                    local_hash_opt = None;
                                } else {
                                    // Hashes were equal; here we ignore hash collisions
                                    i += 1;
                                    j += 1;
                                    remote_hash_opt = None;
                                    local_hash_opt = None;
                                }
                            }

                            // Iterate over the remaining array (since the other one is empty now)
                            while i < remote_data.len() {
                                add.push(unsafe { remote_data.get_unchecked(i) });
                                i += 1;
                            }

                            while j < local_data.len() {
                                remove.push(unsafe { local_data.get_unchecked(j) });
                                j += 1;
                            }

                            // The remote wasn't dumping its set, thus it was below the threshold
                            // Meaning we didn't get *all* of its elements
                            if !remote_dump {
                                new_view.push(local_set);
                            }
                        }
                    }
                }
            }
        }
        Ok(Round {
            add: add,
            remove: remove,
            view: new_view,
        })
    }
}

#[cfg(test)]
mod tests {

    use std::collections::HashSet;

    use super::*;

    use rand::Rng;

    const NUM_ITERS: u32 = 50000;
    #[test]
    fn get_returns_in_order() {
        let mut set = SyncSet::new();

        for i in 0..NUM_ITERS {
            set.insert(i).unwrap();
        }

        assert_eq!(set.root.size(), NUM_ITERS as usize, "Root has wrong size");

        if let Set::ListSet { underlying, .. } = set.get(&Prefix::empty(), true).unwrap() {
            let mut previous = hash(underlying.get(0).expect("get() returns no elements")).unwrap();
            for i in 1..NUM_ITERS {
                let current = hash(underlying.get(i as usize).unwrap())
                    .expect("get() returns too few elements");
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
        let arbitrary_elem = rand::random::<u32>() % NUM_ITERS;
        let arbitrary_non_elem = rand::random::<u32>() % NUM_ITERS + NUM_ITERS;

        for i in 0..NUM_ITERS {
            syncset.insert(i).unwrap();
        }

        let arbitrary_elem_path = Path::new(&arbitrary_elem).unwrap();
        let arbitrary_non_elem_path = Path::new(&arbitrary_non_elem).unwrap();
        assert_ne!(
            arbitrary_elem_path, arbitrary_non_elem_path,
            "The extremely unlikely case of a hash collision has occurred"
        );

        for depth in 0..Path::NUM_BITS {
            {
                // Test if the element is contained
                let expected_prefix = arbitrary_elem_path.prefix(depth);
                let set = syncset.get(&expected_prefix, false).unwrap();
                match &set {
                    Set::LabelSet { prefix, label } => {
                        assert!(prefix.is_prefix_of(&arbitrary_elem_path));
                        assert_eq!(prefix, &expected_prefix, "Returned path does not match prefix");
                        if let n @ Node::Internal { .. } = syncset.root.node_at(&prefix, 0) {
                            assert_eq!(&n.label().unwrap(), label);
                        } else {
                            panic!("get returns a labelset of a leaf or empty, {:?}", set)
                        }
                    }
                    Set::ListSet {
                        underlying,
                        prefix,
                        dump,
                    } => {
                        assert!(
                            underlying.len() <= super::super::DUMP_THRESHOLD,
                            "Number of elements received exceeds the threshold"
                        );
                        assert!(
                            prefix.is_prefix_of(&arbitrary_elem_path),
                            "Prefix isn't a prefix of the full hash"
                        );
                        assert_eq!(prefix, &expected_prefix, "Prefix doesn't match expected");
                        assert!(!dump, "get returns wrong value for dump");
                        let mut success = false;
                        for elem in underlying {
                            if *elem == &arbitrary_elem {
                                success = true
                            }
                        }

                        assert!(
                            success,
                            "Arbitrarily chosen element not found in the ListSet"
                        );
                    }
                }
            }

            // Test if the non-element is not contained
            {
                let expected_prefix = arbitrary_non_elem_path.prefix(depth);
                let set = syncset.get(&expected_prefix, false).unwrap();
                match &set {
                    Set::LabelSet { prefix, label } => {
                        assert!(prefix.is_prefix_of(&arbitrary_non_elem_path));
                        assert_eq!(prefix, &expected_prefix, "Returned path does not match prefix");
                        if let n @ Node::Internal { .. } = syncset.root.node_at(&prefix, 0) {
                            assert_eq!(&n.label().unwrap(), label);
                        } else {
                            panic!("get returns a labelset of a leaf or empty, {:?}", set)
                        }
                    }
                    Set::ListSet {
                        underlying,
                        prefix,
                        dump,
                    } => {
                        assert!(
                            underlying.len() <= super::super::DUMP_THRESHOLD,
                            "Number of elements received exceeds the threshold"
                        );
                        assert!(
                            prefix.is_prefix_of(&arbitrary_non_elem_path),
                            "Prefix isn't a prefix of the full hash"
                        );
                        assert_eq!(prefix, &expected_prefix, "Prefix doesn't match expected");
                        assert!(!dump, "get returns wrong value for dump");

                        for elem in underlying {
                            assert_ne!(*elem, &arbitrary_non_elem, "Non-elem found in get()");
                        }
                    }
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
                expected_size += 1;
                set.insert(i);
                syncset.insert(i).unwrap();
            }
        }

        assert_eq!(syncset.size(), expected_size, "syncset has wrong size");
        for i in 0..2 * NUM_ITERS {
            let should_find = set.contains(&i);
            let found = syncset.contains(&i).unwrap();
            assert_eq!(
                should_find, found,
                "Element {} present in only one of the sets",
                i
            );
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
                expected_size -= 1;
            }
        }

        assert_eq!(syncset.size(), expected_size, "Syncset has wrong size");

        for i in 0..2 * NUM_ITERS {
            let should_find = set.contains(&i);
            let found = set.contains(&i);
            assert_eq!(
                should_find, found,
                "Element {} present in only one of the sets",
                i
            );
        }
    }

    #[test]
    fn sync() {

        type Set = HashSet<u32>;
        let mut alice = SyncSet::new();
        let mut bob = SyncSet::new();
        for i in 0..NUM_ITERS {
            assert!(alice.insert(i).unwrap(), "Inserting element {} fails", i);
            assert!(bob.insert(i).unwrap(), "Inserting element {} fails", i);
        }

        let mut generator = rand::thread_rng();
        let num_extra_elems = 5;
        let mut expected_diff_alice = Set::new();
        let mut expected_diff_bob = Set::new();

        let mut elems_alice_thinks_bob_hasnt = Set::new();
        let mut elems_alice_thinks_bob_has = Set::new();
        let mut elems_bob_thinks_alice_has = Set::new();
        let mut elems_bob_thinks_alice_hasnt = Set::new();

        for i in NUM_ITERS..num_extra_elems + NUM_ITERS {
            if generator.gen() {
                expected_diff_alice.insert(i);
                assert!(alice.insert(i).unwrap(), "Inserting element {} fails", i);
            } else {
                expected_diff_bob.insert(i);
                assert!(bob.insert(i).unwrap(), "Inserting element {} fails", i)
            }
        }

        assert_ne!(
            alice.get(&Prefix::empty(), true).unwrap(),
            bob.get(&Prefix::empty(), true).unwrap(),
            "Alice and Bob shouldn't have the same elements"
        );
        {
            let init_round = alice.start_sync().unwrap();
            let mut view: Vec<_> = init_round.view.iter().map(|e| e.obtain_ownership()).collect();
            let mut alice_turn = false;
            while view.len() != 0 {
                let round = if alice_turn {
                    let round = alice.sync(&view).unwrap();
                    insert_all(&mut elems_alice_thinks_bob_hasnt, &round.remove);
                    insert_all(&mut elems_alice_thinks_bob_has, &round.add);
                    round
                } else {
                    let round = bob.sync(&view).unwrap();
                    insert_all(&mut elems_bob_thinks_alice_hasnt, &round.remove);
                    insert_all(&mut elems_bob_thinks_alice_has, &round.add);
                    round
                };


                // Copy over the elements owned by the sets
                // Normally this would happen as a serialize -> network -> deserialize process
                /*
                view.clear();
                for elem in &round.view {
                    let to_insert = elem.obtain_ownership();
                    view.push(to_insert);
                }*/
                view = round.view.iter().map(|e| e.obtain_ownership()).collect();
                alice_turn = !alice_turn
            }
        }

        assert_eq!(
            elems_alice_thinks_bob_has, elems_bob_thinks_alice_hasnt,
            "Alice and bob don't agree on bob's elements"
        );

        assert_eq!(
            elems_alice_thinks_bob_hasnt, elems_bob_thinks_alice_has,
            "Alice and bob don't agree on alice's elements"
        );

        assert_eq!(
            elems_alice_thinks_bob_has, expected_diff_bob,
            "Bob's elements don't match expectations"
        );

        assert_eq!(
            elems_bob_thinks_alice_has, expected_diff_alice,
            "Alice's elements don't match expectations"
        );

        for elem in &expected_diff_bob {
            assert!(alice.insert(*elem).unwrap());
        }

        // Bob is now a subset of alice
        let mut diff = Set::new();
        {
            let init_round = alice.start_sync().unwrap();
            let mut view: Vec<_> = init_round.view.iter().map(|e| e.obtain_ownership()).collect();
            let mut alice_turn = false;
            while view.len() != 0 {

                let round = if alice_turn {
                    let round = alice.sync(&view).unwrap();
                    assert!(round.add.is_empty(), "Round add isn't empty for alice");
                    insert_all(&mut diff, &round.remove);
                    round
                } else {
                    let round = bob.sync(&view).unwrap();
                    assert!(round.remove.is_empty(), "Round remove isn't empty for bob");
                    insert_all(&mut diff, &round.add);
                    round
                };
                // Copy over the elements owned by the sets
                // Normally this would happen as a serialize -> network -> deserialize process
                view = round.view.iter().map(|e| e.obtain_ownership()).collect();
                alice_turn = !alice_turn
            }
        }

        assert_eq!(
            diff, expected_diff_alice,
            "Bob's elements don't match expectations"
        );
    }

    fn insert_all<T: Eq + std::hash::Hash + Clone>(left: &mut HashSet<T>, right: &Vec<&T>) {
        for elem in right {
            left.insert((*elem).clone());
        }
    }
}
