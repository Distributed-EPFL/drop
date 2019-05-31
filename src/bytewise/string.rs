// Dependencies

use crate::data::Varint;
use failure::Error;
use std::str;
use super::load::Load;
use super::readable::Readable;
use super::reader::Reader;
use super::size::Size;
use super::writable::Writable;
use super::writer::Writer;

// Implementations

impl Readable for String {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Error> {
        visitor.visit(&Varint(self.len() as u32))?;
        visitor.push(self.as_bytes())
    }
}

impl Writable for String {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Error> {
        let size = Varint::load(visitor)?.0 as usize;

        self.clear();
        self.reserve(size);

        self.push_str(str::from_utf8(visitor.pop(size)?)?);
        Ok(())
    }
}

impl Load for String {
    fn load<From: Writer>(from: &mut From) -> Result<Self, Error> {
        let mut string = String::new();
        from.visit(&mut string)?;
        Ok(string)
    }
}
