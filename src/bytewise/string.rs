// Dependencies

use crate::data::Varint;
use std::str;
use super::errors::ReadError;
use super::errors::WritableError;
use super::errors::WriteError;
use super::load::Load;
use super::readable::Readable;
use super::reader::Reader;
use super::size::Size;
use super::writable::Writable;
use super::writer::Writer;

// Implementations

impl Readable for String {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.visit(&Varint(self.len() as u32))?;
        visitor.push(self.as_bytes())?;
        Ok(())
    }
}

impl Writable for String {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        let size = Varint::load(visitor)?.0 as usize;

        self.clear();
        self.reserve(size);

        let buffer = visitor.pop(size)?;
        let buffer = match str::from_utf8(buffer) {
            Ok(buffer) => buffer,
            Err(_) => return Err(WritableError::new("Utf8Error").into())
        };

        self.push_str(buffer);
        Ok(())
    }
}

impl Load for String {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        let mut string = String::new();
        from.visit(&mut string)?;
        Ok(string)
    }
}
