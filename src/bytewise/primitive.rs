// Dependencies

use std::convert::TryInto;
use super::readable::Readable;
use super::reader::Reader;
use super::size::Size;
use super::writable::Writable;
use super::writer::Writer;

// Implementations

impl Readable for bool {
    const SIZE: Size = Size::Fixed(1);

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
        if *self {
            visitor.push(&[0x01])
        } else {
            visitor.push(&[0x00])
        }
    }
}

impl Writable for bool {
    const SIZE: Size = Size::Fixed(1);

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
        *self = visitor.pop(1)?[0] != 0;
        Ok(())
    }
}

macro_rules! implement {
    ($($type:ty: $size:expr), *) => ($(
        impl Readable for $type {
            const SIZE: Size = Size::Fixed($size);

            fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
                visitor.push(&self.to_le_bytes())
            }
        }

        impl Writable for $type {
            const SIZE: Size = Size::Fixed($size);

            fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
                *self = Self::from_le_bytes(visitor.pop($size)?.try_into().unwrap());
                Ok(())
            }
        }
    )*);
}

implement!(i8: 1, i16: 2, i32: 4, i64: 8, i128: 16, u8: 1, u16: 2, u32: 4, u64: 8, u128: 16);
