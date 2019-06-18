// Dependencies

use std::vec::Vec;
use super::errors::ReadError;
use super::errors::ReaderError;
use super::measurable::Measurable;
use super::readable::Readable;
use super::reader::Reader;

// Structs

pub struct Serializer<'s>(&'s mut Vec<u8>);

// Implementations

impl Reader for Serializer<'_> {
    fn push(&mut self, chunk: &[u8]) -> Result<(), ReaderError> {
        self.0.extend_from_slice(chunk);
        Ok(())
    }
}

// Functions

pub fn serialize<Acceptor: Readable>(acceptor: &Acceptor) -> Result<Vec<u8>, ReadError> {
    let mut vec = Vec::with_capacity(acceptor.size()?);
    let mut serializer = Serializer(&mut vec);
    serializer.visit(acceptor)?;
    Ok(vec)
}

// Test

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::*;
    use super::super::errors::ReadableError;
    use super::super::size::Size;

    // Structs

    struct Unreadable;

    // Implementations

    impl Readable for Unreadable {
        const SIZE: Size = Size::variable();
        fn accept<Visitor: Reader>(&self, _: &mut Visitor) -> Result<(), ReadError> {
            Err(ReadableError::new("IAmUnreadable").into())
        }
    }

    // Test cases

    #[test]
    fn reference() {
        assert_eq!(serialize::<[u32; 4]>(&[0x01, 0x02, 0x03, 0x04]).unwrap(), vec![0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00]);
        serialize(&Unreadable).unwrap_err();
    }
}
