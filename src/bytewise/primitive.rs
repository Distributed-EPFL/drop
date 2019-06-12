// Dependencies

use std::convert::TryInto;
use super::errors::ReadError;
use super::errors::WriteError;
use super::load::Load;
use super::readable::Readable;
use super::reader::Reader;
use super::size::Size;
use super::writable::Writable;
use super::writer::Writer;

// Implementations

impl Readable for bool {
    const SIZE: Size = Size::fixed(1);

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
        visitor.push(&[*self as u8])?;
        Ok(())
    }
}

impl Writable for bool {
    const SIZE: Size = Size::fixed(1);

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
        *self = Self::load(visitor)?;
        Ok(())
    }
}

impl Load for bool {
    fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
        Ok(from.pop(1)?[0] != 0)
    }
}

macro_rules! implement {
    ($($type:ty: $size:expr), *) => ($(
        impl Readable for $type {
            const SIZE: Size = Size::fixed($size);

            fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
                visitor.push(&self.to_le_bytes())?;
                Ok(())
            }
        }

        impl Writable for $type {
            const SIZE: Size = Size::fixed($size);

            fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
                *self = Self::load(visitor)?;
                Ok(())
            }
        }

        impl Load for $type {
            fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
                Ok(Self::from_le_bytes(from.pop($size)?.try_into().unwrap()))
            }
        }
    )*);
}

implement!(i8: 1, i16: 2, i32: 4, i64: 8, i128: 16, u8: 1, u16: 2, u32: 4, u64: 8, u128: 16);

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::*;
    use super::super::load::Load;
    use super::super::errors::ReaderError;
    use super::super::errors::WriterError;

    // Structs

    struct Reference(&'static [u8]);

    // Implementations

    impl Reader for Reference {
        fn push(&mut self, chunk: &[u8]) -> Result<(), ReaderError> {
            if self.0 == chunk { Ok(()) } else { Err(ReaderError::new("UnexpectedInput")) }
        }
    }

    impl Writer for Reference {
        fn pop(&mut self, size: usize) -> Result<&[u8], WriterError> {
            if size == self.0.len() { Ok(self.0) } else { Err(WriterError::new("UnexpectedSize").into()) }
        }
    }

    // Macros

    macro_rules! testcase {
        ($type:ty, $value:expr, $reference:expr) => {
            let value: $type = $value;
            Readable::accept(&value, &mut Reference(&$reference[..])).unwrap();

            let value = <$type>::load(&mut Reference(&$reference[..])).unwrap();
            assert_eq!(value, $value);
        }
    }

    // Test cases

    #[test]
    fn boolean() {
        testcase!(bool, false, [0]);
        testcase!(bool, true, [1]);
    }

    #[test]
    fn integers() {
        testcase!(i8, 0x07, [0x07]);
        testcase!(u8, 0x17, [0x17]);

        testcase!(i16, 0x0716, [0x16, 0x07]);
        testcase!(u16, 0x2716, [0x16, 0x27]);

        testcase!(i32, 0x07164a22, [0x22, 0x4a, 0x16, 0x07]);
        testcase!(u32, 0x37164a22, [0x22, 0x4a, 0x16, 0x37]);

        testcase!(i64, 0x07164a225c19057b, [0x7b, 0x05, 0x19, 0x5c, 0x22, 0x4a, 0x16, 0x07]);
        testcase!(u64, 0x47164a225c19057b, [0x7b, 0x05, 0x19, 0x5c, 0x22, 0x4a, 0x16, 0x47]);

        testcase!(i128, 0x07164a225c19057b07164a226cbbaa8c, [0x8c, 0xaa, 0xbb, 0x6c, 0x22, 0x4a, 0x16, 0x07, 0x7b, 0x05, 0x19, 0x5c, 0x22, 0x4a, 0x16, 0x07]);
        testcase!(u128, 0x57164a225c19057b07164a226cbbaa8c, [0x8c, 0xaa, 0xbb, 0x6c, 0x22, 0x4a, 0x16, 0x07, 0x7b, 0x05, 0x19, 0x5c, 0x22, 0x4a, 0x16, 0x57]);
    }
}
