// Dependencies

use crate::data::Varint;
use std::vec::Vec;
use super::errors::ReadError;
use super::errors::WriteError;
use super::load::Load;
use super::readable::Readable;
use super::reader::Reader;
use super::size::Size;
use super::writable::Writable;
use super::writer::Writer;

// Implementations

impl<Item: Readable> Readable for Vec<Item> {
    const SIZE: Size = Size::variable();

    default fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.visit(&Varint(self.len() as u32))?;

        for item in self {
            visitor.visit(item)?;
        }

        Ok(())
    }
}

impl<Item: Load> Writable for Vec<Item> {
    const SIZE: Size = Size::variable();

    default fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let size = Varint::load(visitor)?.0 as usize;

        self.clear();
        self.reserve(size);

        for _ in 0..size {
            self.push(Item::load(visitor)?);
        }

        Ok(())
    }
}

impl<Item: Load> Load for Vec<Item> {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        let mut vec = Vec::<Item>::new();
        from.visit(&mut vec)?;
        Ok(vec)
    }
}

impl Readable for Vec<u8> {
    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.visit(&Varint(self.len() as u32))?;
        visitor.push(self)?;
        Ok(())
    }
}

impl Writable for Vec<u8> {
    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let size = Varint::load(visitor)?.0 as usize;

        self.clear();
        self.extend_from_slice(visitor.pop(size)?);

        Ok(())
    }
}
