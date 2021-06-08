use crate::crypto::Digest;
use crate::crypto::hash;

use serde::Serialize;

use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};

use super::batch::{Action, Batch, Task};
use super::database::Store;
use super::entry::{Entry as StoreEntry, Node};
use super::label::{label, EMPTY};
use super::prefix::{Direction, Path};

#[derive(Copy, Clone)]
enum Brief {
    Empty,
    Internal(Digest),
    Leaf(Digest)
}

enum References {
    Applicable(usize),
    NotApplicable
}

struct Entry<Key: Serialize, Value: Serialize> {
    label: Digest,
    node: Node<Key, Value>,
    references: References
}

impl Brief {
    fn label(&self) -> &Digest {
        match self {
            Brief::Empty => &EMPTY,
            Brief::Internal(label) => label,
            Brief::Leaf(label) => label
        }
    }
}

impl References {
    fn multiple(&self) -> bool {
        match self {
            References::Applicable(references) => *references > 1,
            References::NotApplicable => false
        }
    }
}

impl<Key, Value> Entry<Key, Value>
where
    Key: Serialize,
    Value: Serialize
{
    fn empty() -> Self {
        Entry{label: EMPTY, node: Node::Empty, references: References::NotApplicable}
    }

    fn brief(&self) -> Brief {
        match self.node {
            Node::Empty => Brief::Empty,
            Node::Internal(..) => Brief::Internal(self.label),
            Node::Leaf(..) => Brief::Leaf(self.label)
        }
    }
}

fn get<Key, Value>(store: &Store<Key, Value>, label: Digest) -> Entry<Key, Value>
where
    Key: Serialize,
    Value: Serialize
{
    if label == EMPTY {
        Entry{label: EMPTY, node: Node::Empty, references: References::NotApplicable}
    } else {
        let entry = &store.entries[&label];
        Entry{label, node: entry.node.clone(), references: References::Applicable(entry.references)}
    }
}

fn incref<Key, Value>(store: &mut Store<Key, Value>, label: &Digest, node: Node<Key, Value>) 
where
    Key: Serialize,
    Value: Serialize
{
    if *label != EMPTY {
        match store.entries.get_mut(label) {
            Some(entry) => {
                entry.references += 1;

                // This `match` is tied to the traversal of a `MerkleTable`'s tree: 
                // increfing an internal node implies a previous incref of its children, 
                // which needs to be correct upon deduplication. 
                // A normal `incref` method would not have this.
                match node { 
                    Node::Internal(left, right) => {
                        store.entries.get_mut(&left).unwrap().references -= 1;
                        store.entries.get_mut(&right).unwrap().references -= 1;
                    },
                    _ => {}
                }
            },
            None => {
                store.entries.insert(*label, StoreEntry{node, references: 1});
            }
        }
    }
}

fn decref<Key, Value>(store: &mut Store<Key, Value>, label: &Digest) 
where
    Key: Serialize,
    Value: Serialize
{
    if *label != EMPTY {
        match store.entries.entry(*label) {
            Occupied(mut entry) => {
                let value = entry.get_mut();
                value.references -= 1;

                if value.references == 0 {
                    entry.remove_entry();
                }
            }
            Vacant(_) => {}
        };
    }
}

fn branch<Key, Value>(store: &mut Store<Key, Value>, original: Option<&Entry<Key, Value>>, preserve: bool, depth: u8, batch: Batch<Key, Value>, left: Entry<Key, Value>, right: Entry<Key, Value>) -> Brief 
where
    Key: Serialize,
    Value: Serialize
{
    let preserve_branches = preserve || if let Some(original) = original { original.references.multiple() } else { false };

    let left = recur(store, left, preserve_branches, depth + 1, batch.left());
    let right = recur(store, right, preserve_branches, depth + 1, batch.right());

    let new = match (left, right) {
        (Brief::Empty, Brief::Empty) => Brief::Empty,
        (Brief::Empty, Brief::Leaf(label)) | (Brief::Leaf(label), Brief::Empty) => Brief::Leaf(label),
        (left, right) => {
            let node = Node::<Key, Value>::Internal(*left.label(), *right.label());
            match original {
                Some(original) if node == original.node => { // Unchanged `original`
                    original.brief()
                }
                _ => { // New or modified `original`
                    let label = label(&node);
                    incref(store, &label, node);
                    Brief::Internal(label)
                }
            }
        }
    };

    if let Some(original) = original {
        if *new.label() != original.label && !preserve {
            decref(store, &original.label);
        }
    }

    new
}

fn recur<Key, Value>(store: &mut Store<Key, Value>, target: Entry<Key, Value>, preserve: bool, depth: u8, batch: Batch<Key, Value>) -> Brief 
where
    Key: Serialize,
    Value: Serialize
{
    match (&target.node, batch.task()) {
        (_, Task::Pass) => target.brief(),

        (Node::Empty, Task::Do(operation)) => {
            match &operation.action {
                Action::Set(value) => {
                    let node = Node::Leaf(operation.key.clone(), value.clone());
                    let label = label(&node);
                    incref(store, &label, node);

                    Brief::Leaf(label)
                },
                Action::Remove => Brief::Empty
            }
        },
        (Node::Empty, Task::Split) => branch(store, None, preserve, depth, batch, Entry::empty(), Entry::empty()),

        (Node::Leaf(key, original_value), Task::Do(operation)) if *key == operation.key => {
            match &operation.action {
                Action::Set(new_value) if new_value != original_value => {
                    let node = Node::Leaf(operation.key.clone(), new_value.clone());
                    let label = label(&node);
                    incref(store, &label, node);

                    if !preserve {
                        decref(store, &target.label);
                    }

                    Brief::Leaf(label)
                },
                Action::Set(_) => target.brief(),
                Action::Remove => {
                    if !preserve {
                        decref(store, &target.label);
                    }

                    Brief::Empty
                }
            }
        }
        (Node::Leaf(key, value), _) => {
            let (left, right) = if Path::new(target.label)[depth] == Direction::Left {
                (target, Entry::empty())
            } else {
                (Entry::empty(), target)
            };

            branch(store, None, preserve, depth, batch, left, right)
        },

        (Node::Internal(left, right), _) => branch(store, Some(&target), preserve, depth, batch, get(store, *left), get(store, *right))
    }
}

pub(super) fn traverse<Key, Value>(store: &mut Store<Key, Value>, root: Digest, batch: Batch<Key, Value>) 
where
    Key: Serialize,
    Value: Serialize
{
    recur(store, get(store, root), false, 0, batch);
}