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

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use rand;
    use rand::Rng;
    use rand::distributions::Alphanumeric;
    use std::iter;
    use super::*;
    use super::super::testing::invert;
    use super::super::testing::reference;
    use super::super::testing::reference::Buffer;

    #[test]
    fn reference() {
        reference::all(&"".to_string(), &[0x00]);
        reference::all(&"Hello World".to_string(), &[0x0b, 0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x57, 0x6f, 0x72, 0x6c, 0x64]);

        let mut string = String::new();
        Reader::visit(&mut Buffer::new(&[0x01, 0x00]), &mut string).unwrap_err();
    }

    #[test]
    fn invert() {
        let mut rng = rand::thread_rng();

        for _ in 0..128 {
            let size = rng.gen_range(0, 256);
            let string: String = iter::repeat(()).map(|()| rng.sample(Alphanumeric)).take(size).collect();
            invert::invert(string, |value, reference| assert_eq!(value, reference));
        }
    }
}
