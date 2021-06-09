use crate::crypto::hash;
use crate::crypto::hash::HashError;

use serde::Serialize;

use std::rc::Rc;

use super::entry::Wrap;
use super::prefix::{Path, Prefix};

#[derive(Debug, Eq)]
pub(super) enum Action<Value: Serialize> {
    Set(Wrap<Value>),
    Remove
}

#[derive(Debug, Eq)]
pub(super) struct Operation<Key: Serialize, Value: Serialize> {
    pub path: Path,
    pub key: Wrap<Key>,
    pub action: Action<Value>
}

pub(super) struct Batch<'a, Key: Serialize, Value: Serialize> {
    prefix: Prefix,
    operations: &'a [Operation<Key, Value>]
}

#[derive(Eq, Debug)]
pub(super) enum Task<'a, Key: Serialize, Value: Serialize> {
    Pass,
    Do(&'a Operation<Key, Value>),
    Split
}

impl<Value> PartialEq for Action<Value>
where Value: Serialize
{
    fn eq(&self, rho: &Self) -> bool {
        match (self, rho) {
            (Action::Set(self_value), Action::Set(rho_value)) => self_value == rho_value,
            (Action::Remove, Action::Remove) => true,
            _ => false
        }
    }
}

impl<Key, Value> PartialEq for Operation<Key, Value> 
where
    Key: Serialize,
    Value: Serialize
{
    fn eq(&self, rho: &Self) -> bool {
        (self.key == rho.key) && (self.action == rho.action) // `path` is uniquely determined by `key`
    }
}

impl<'a, Key, Value> PartialEq for Task<'a, Key, Value>
where
    Key: Serialize,
    Value: Serialize
{
    fn eq(&self, rho: &Self) -> bool {
        match (self, rho) {
            (Task::Pass, Task::Pass) => true,
            (Task::Do(self_op), Task::Do(rho_op)) => self_op == rho_op,
            (Task::Split, Task::Split) => true,
            _ => false
        }
    }
}

impl<Key, Value> Operation<Key, Value> 
where 
    Key: Serialize,
    Value: Serialize
{
    fn set(key: Key, value: Value) -> Result<Self, HashError> {
        let key = Wrap::new(key)?;
        let value = Wrap::new(value)?;
        
        Ok(Operation{path: Path::from(key.digest()), key, action: Action::Set(value)})
    }

    fn remove(key: Key) -> Result<Self, HashError> {
        let key = Wrap::new(key)?;
        Ok(Operation{path: Path::from(key.digest()), key, action: Action::Remove})
    }
}

impl<'a, Key, Value> Batch<'a, Key, Value> 
where
    Key: Serialize,
    Value: Serialize
{
    pub fn new(operations: &'a mut [Operation<Key, Value>]) -> Self {
        operations.sort_unstable_by(|lho, rho| lho.path.cmp(&rho.path));
        Batch{prefix: Prefix::root(), operations}
    }

    pub fn prefix(&self) -> &Prefix {
        &self.prefix
    }

    pub fn task(&self) -> Task<Key, Value> {
        match self.operations.len() {
            0 => Task::Pass,
            1 => Task::Do(&self.operations[0]),
            _ => Task::Split
        }
    }

    pub fn left(&self) -> Self {
        Batch{prefix: self.prefix.left(), operations: &self.operations[self.partition()..]}
    }

    pub fn right(&self) -> Self {
        Batch{prefix: self.prefix.right(), operations: &self.operations[..self.partition()]}
    }

    fn partition(&self) -> usize {
        let right = self.prefix.right();
        self.operations.partition_point(|operation| right.contains(&operation.path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::prefix::Prefix;

    use std::vec::Vec;

    fn split_recursion(batch: &Batch<u32, u32>) -> (u32, bool) {
        match batch.task() {
            Task::Pass => (0, true),
            Task::Do(operation) => {
                if batch.prefix().contains(&operation.path) { (1, true) } else { (1, false) }
            },
            Task::Split => {
                let (lcount, lpass) = split_recursion(&batch.left());
                let (rcount, rpass) = split_recursion(&batch.right());

                (lcount + rcount, lpass && rpass)
            }
        }
    }

    #[test]
    fn operation() {
        let prefix = Prefix::root().left().left().left().right().left().left().right().right().right().right().left().right().left().right().left().left();
        
        let set = Operation::set(0u32, 8u32).unwrap();
        assert!(prefix.contains(&set.path));
        assert_eq!(set.key, Wrap::new(0u32).unwrap());
        assert_eq!(set.action, Action::Set(Wrap::new(8u32).unwrap()));

        let remove = Operation::remove(0u32).unwrap();
        assert_eq!(remove.path, set.path);
        assert_eq!(remove.key, set.key);
        assert_eq!(remove.action, Action::<u32>::Remove);
    }

    #[test]
    fn prefix() {
        let mut operations: Vec<Operation<u32, u32>> = Vec::new();
        let batch = Batch::new(&mut operations);

        assert_eq!(batch.prefix(), &Prefix::root());
        assert_eq!(batch.left().prefix(), &Prefix::root().left());
        assert_eq!(batch.right().right().right().left().right().right().right().prefix(), &Prefix::root().right().right().right().left().right().right().right());
    }

    #[test]
    fn task_develop() {
        let mut operations: Vec<Operation<u32, u32>> = (0u32..4u32).map(|index| Operation::set(index, index).unwrap()).collect();
        let batch = Batch::new(&mut operations);

        assert_eq!(batch.task(), Task::Split);

        assert_eq!(batch.left().task(), Task::Split);
        assert_eq!(batch.right().task(), Task::Pass);

        assert_eq!(batch.left().left().task(), Task::Split);
        assert_eq!(batch.left().right().task(), Task::Do(&Operation::set(3u32, 3u32).unwrap()));
        
        assert_eq!(batch.left().left().left().task(), Task::Split);
        assert_eq!(batch.left().left().right().task(), Task::Do(&Operation::set(1u32, 1u32).unwrap()));

        assert_eq!(batch.left().left().left().left().task(), Task::Do(&Operation::set(2u32, 2u32).unwrap()));
        assert_eq!(batch.left().left().left().right().task(), Task::Do(&Operation::set(0u32, 0u32).unwrap()));
    }

    #[test]
    fn distribution() {
        let mut operations: Vec<Operation<u32, u32>> = (0u32..64u32).map(|index| Operation::set(index, index).unwrap()).collect();
        let batch = Batch::new(&mut operations);

        assert_eq!(split_recursion(&batch), (64, true));
    }
}