// Dependencies

use crate::data::Varint;
use failure::Error;
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
use super::load::Load;
use super::readable::Readable;
use super::reader::Reader;
use super::size::Size;
use super::writable::Writable;
use super::writer::Writer;

// Implementations

impl<Item: Readable> Readable for BinaryHeap<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load + Ord> Writable for BinaryHeap<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Error> {
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
    fn load<From: Writer>(from: &mut From) -> Result<Self, Error> {
        let mut heap = BinaryHeap::<Item>::new();
        from.visit(&mut heap)?;
        Ok(heap)
    }
}

impl<Key: Readable, Value: Readable> Readable for BTreeMap<Key, Value> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Error> {
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

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Error> {
        let size = Varint::load(visitor)?.0 as usize;
        self.clear();

        for _ in 0..size {
            self.insert(Key::load(visitor)?, Value::load(visitor)?);
        }

        Ok(())
    }
}

impl<Key: Load + Ord, Value: Load> Load for BTreeMap<Key, Value> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, Error> {
        let mut map = BTreeMap::<Key, Value>::new();
        from.visit(&mut map)?;
        Ok(map)
    }
}

impl<Item: Readable> Readable for BTreeSet<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load + Ord> Writable for BTreeSet<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Error> {
        let size = Varint::load(visitor)?.0 as usize;
        self.clear();

        for _ in 0..size {
            self.insert(Item::load(visitor)?);
        }

        Ok(())
    }
}

impl<Item: Load + Ord> Load for BTreeSet<Item> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, Error> {
        let mut set = BTreeSet::<Item>::new();
        from.visit(&mut set)?;
        Ok(set)
    }
}

impl<Key: Readable, Value: Readable> Readable for HashMap<Key, Value> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Error> {
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

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Error> {
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
    fn load<From: Writer>(from: &mut From) -> Result<Self, Error> {
        let mut map = HashMap::<Key, Value>::new();
        from.visit(&mut map)?;
        Ok(map)
    }
}

impl<Item: Readable> Readable for HashSet<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load + Eq + Hash> Writable for HashSet<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Error> {
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
    fn load<From: Writer>(from: &mut From) -> Result<Self, Error> {
        let mut set = HashSet::<Item>::new();
        from.visit(&mut set)?;
        Ok(set)
    }
}

impl<Item: Readable> Readable for LinkedList<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load> Writable for LinkedList<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Error> {
        let size = Varint::load(visitor)?.0 as usize;
        self.clear();

        for _ in 0..size {
            self.push_back(Item::load(visitor)?);
        }

        Ok(())
    }
}

impl<Item: Load> Load for LinkedList<Item> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, Error> {
        let mut list = LinkedList::<Item>::new();
        from.visit(&mut list)?;
        Ok(list)
    }
}

impl<Item: Readable> Readable for VecDeque<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load> Writable for VecDeque<Item> {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Error> {
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
    fn load<From: Writer>(from: &mut From) -> Result<Self, Error> {
        let mut deque = VecDeque::<Item>::new();
        from.visit(&mut deque)?;
        Ok(deque)
    }
}
