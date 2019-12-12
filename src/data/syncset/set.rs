use super::node::Node;
use super::path::Prefix;
use super::Syncable;
use crate::crypto::hash::Digest;

/// Data structure used to synchronize two SyncSets
#[derive(Debug, PartialEq, Clone)]
pub enum Set<Data> {
    /// Lightweight alternative, only contains the hash of
    /// the sub-tree at prefix
    LabelSet { prefix: Prefix, label: Digest },

    /// Heavy alternative, contains all the data of a sub-tree at
    /// a given prefix
    ListSet {
        underlying: Vec<Data>,
        prefix: Prefix,
        dump: bool,
    },
}

impl<Data: Syncable> Set<Data> {
    // Constructors, for ease of use
    pub(super) fn new_dataset(
        prefix: Prefix,
        node: &Node<Data>,
        dump: bool,
    ) -> Set<&Data> {
        let underlying = node.dump();

        Set::ListSet {
            underlying,
            prefix,
            dump,
        }
    }

    pub(super) fn new_empty_dataset(prefix: Prefix, dump: bool) -> Set<Data> {
        let underlying = Vec::new();
        Set::ListSet {
            underlying,
            prefix,
            dump,
        }
    }
}

impl<Data: Syncable + Clone> Set<&Data> {
    /// Clones the inner elements to obtain a Set that owns its data
    pub fn obtain_ownership(&self) -> Set<Data> {
        use Set::*;
        match self {
            LabelSet { prefix, label } => LabelSet {
                prefix: prefix.clone(),
                label: label.clone(),
            },
            ListSet {
                underlying,
                prefix,
                dump,
            } => {
                let mut new_underlying: Vec<Data> =
                    Vec::with_capacity(underlying.len());
                for elem in underlying {
                    new_underlying.push((*elem).clone());
                }

                ListSet {
                    underlying: new_underlying,
                    prefix: prefix.clone(),
                    dump: *dump,
                }
            }
        }
    }
}
