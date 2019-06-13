// Dependencies

use crate::data::Varint;
use std::cmp::Eq;
use std::cmp::Ord;
use std::collections::BinaryHeap;
use std::collections::BTreeMap;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::LinkedList;
use std::collections::VecDeque;
use std::hash::Hash;
use super::errors::ReadError;
use super::errors::WriteError;
use super::load::Load;
use super::readable::Readable;
use super::reader::Reader;
use super::size::Size;
use super::writable::Writable;
use super::writer::Writer;

// Implementations

impl<Item: Readable> Readable for BinaryHeap<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load + Ord> Writable for BinaryHeap<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let size = Varint::load(visitor)?.0 as usize;

        self.clear();
        self.reserve(size);

        for _ in 0..size {
            self.push(Item::load(visitor)?);
        }

        Ok(())
    }
}

impl<Item: Load + Ord> Load for BinaryHeap<Item> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        let mut heap = BinaryHeap::<Item>::new();
        from.visit(&mut heap)?;
        Ok(heap)
    }
}

impl<Key: Readable, Value: Readable> Readable for BTreeMap<Key, Value> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.visit(&Varint(self.len() as u32))?;

        for (key, value) in self {
            visitor.visit(key)?;
            visitor.visit(value)?;
        }

        Ok(())
    }
}

impl<Key: Load + Ord, Value: Load> Writable for BTreeMap<Key, Value> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let size = Varint::load(visitor)?.0 as usize;
        self.clear();

        for _ in 0..size {
            self.insert(Key::load(visitor)?, Value::load(visitor)?);
        }

        Ok(())
    }
}

impl<Key: Load + Ord, Value: Load> Load for BTreeMap<Key, Value> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        let mut map = BTreeMap::<Key, Value>::new();
        from.visit(&mut map)?;
        Ok(map)
    }
}

impl<Item: Readable> Readable for BTreeSet<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load + Ord> Writable for BTreeSet<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let size = Varint::load(visitor)?.0 as usize;
        self.clear();

        for _ in 0..size {
            self.insert(Item::load(visitor)?);
        }

        Ok(())
    }
}

impl<Item: Load + Ord> Load for BTreeSet<Item> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        let mut set = BTreeSet::<Item>::new();
        from.visit(&mut set)?;
        Ok(set)
    }
}

impl<Key: Readable, Value: Readable> Readable for HashMap<Key, Value> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.visit(&Varint(self.len() as u32))?;

        for (key, value) in self {
            visitor.visit(key)?;
            visitor.visit(value)?;
        }

        Ok(())
    }
}

impl<Key: Load + Eq + Hash, Value: Load> Writable for HashMap<Key, Value> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let size = Varint::load(visitor)?.0 as usize;

        self.clear();
        self.reserve(size);

        for _ in 0..size {
            self.insert(Key::load(visitor)?, Value::load(visitor)?);
        }

        Ok(())
    }
}

impl<Key: Load + Eq + Hash, Value: Load> Load for HashMap<Key, Value> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        let mut map = HashMap::<Key, Value>::new();
        from.visit(&mut map)?;
        Ok(map)
    }
}

impl<Item: Readable> Readable for HashSet<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load + Eq + Hash> Writable for HashSet<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let size = Varint::load(visitor)?.0 as usize;

        self.clear();
        self.reserve(size);

        for _ in 0..size {
            self.insert(Item::load(visitor)?);
        }

        Ok(())
    }
}

impl<Item: Load + Eq + Hash> Load for HashSet<Item> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        let mut set = HashSet::<Item>::new();
        from.visit(&mut set)?;
        Ok(set)
    }
}

impl<Item: Readable> Readable for LinkedList<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load> Writable for LinkedList<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let size = Varint::load(visitor)?.0 as usize;
        self.clear();

        for _ in 0..size {
            self.push_back(Item::load(visitor)?);
        }

        Ok(())
    }
}

impl<Item: Load> Load for LinkedList<Item> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        let mut list = LinkedList::<Item>::new();
        from.visit(&mut list)?;
        Ok(list)
    }
}

impl<Item: Readable> Readable for VecDeque<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load> Writable for VecDeque<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let size = Varint::load(visitor)?.0 as usize;

        self.clear();
        self.reserve(size);

        for _ in 0..size {
            self.push_back(Item::load(visitor)?);
        }

        Ok(())
    }
}

impl<Item: Load> Load for VecDeque<Item> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        let mut deque = VecDeque::<Item>::new();
        from.visit(&mut deque)?;
        Ok(deque)
    }
}

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use rand;
    use super::*;
    use super::super::testing::invert;

    #[test]
    fn invert() {
        let mut binary_heap = BinaryHeap::<u32>::new();
        for _ in 0..128 { binary_heap.push(rand::random()); }
        invert::invert(binary_heap, |mut value, mut reference| {
            loop {
                match (value.pop(), reference.pop()) {
                    (Some(value), Some(reference)) => assert_eq!(value, reference),
                    (None, None) => return,
                    _ => panic!("`BinaryHeap`s have non-matching lengths.")
                }
            }
        });

        let mut b_tree_map = BTreeMap::<u32, u32>::new();
        for _ in 0..128 { b_tree_map.insert(rand::random(), rand::random()); }
        invert::invert(b_tree_map, |value, reference| {
            assert_eq!(value.len(), reference.len());
            for (key, value) in value {
                assert_eq!(value, reference[&key]);
            }
        });

        let mut b_tree_set = BTreeSet::<u32>::new();
        for _ in 0..128 { b_tree_set.insert(rand::random()); }
        invert::invert(b_tree_set, |value, reference| {
            assert_eq!(value.len(), reference.len());
            for value in value {
                assert!(reference.contains(&value));
            }
        });

        let mut hash_map = HashMap::<u32, u32>::new();
        for _ in 0..128 { hash_map.insert(rand::random(), rand::random()); }
        invert::invert(hash_map, |value, reference| {
            assert_eq!(value.len(), reference.len());
            for (key, value) in value {
                assert_eq!(value, reference[&key]);
            }
        });

        let mut hash_set = HashSet::<u32>::new();
        for _ in 0..128 { hash_set.insert(rand::random()); }
        invert::invert(hash_set, |value, reference| {
            assert_eq!(value.len(), reference.len());
            for value in value {
                assert!(reference.contains(&value));
            }
        });

        let mut linked_list = LinkedList::<u32>::new();
        for _ in 0..128 { linked_list.push_back(rand::random()); }
        invert::invert(linked_list, |value, reference| { assert_eq!(value, reference); });

        let mut vec_deque = VecDeque::<u32>::new();
        for _ in 0..128 { vec_deque.push_back(rand::random()); }
        invert::invert(vec_deque, |value, reference| { assert_eq!(value, reference); });
    }
}
