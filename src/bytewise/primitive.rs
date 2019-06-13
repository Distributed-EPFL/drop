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
    use super::super::testing::reference;

    // Test cases

    #[test]
    fn boolean() {
        reference::all::<bool>(&false, &[0]);
        reference::all::<bool>(&true, &[1]);
    }

    #[test]
    fn integers() {
        reference::all::<i8>(&0x07, &[0x07]);
        reference::all::<u8>(&0x17, &[0x17]);

        reference::all::<i16>(&0x0716, &[0x16, 0x07]);
        reference::all::<u16>(&0x2716, &[0x16, 0x27]);

        reference::all::<i32>(&0x07164a22, &[0x22, 0x4a, 0x16, 0x07]);
        reference::all::<u32>(&0x37164a22, &[0x22, 0x4a, 0x16, 0x37]);

        reference::all::<i64>(&0x07164a225c19057b, &[0x7b, 0x05, 0x19, 0x5c, 0x22, 0x4a, 0x16, 0x07]);
        reference::all::<u64>(&0x47164a225c19057b, &[0x7b, 0x05, 0x19, 0x5c, 0x22, 0x4a, 0x16, 0x47]);

        reference::all::<i128>(&0x07164a225c19057b07164a226cbbaa8c, &[0x8c, 0xaa, 0xbb, 0x6c, 0x22, 0x4a, 0x16, 0x07, 0x7b, 0x05, 0x19, 0x5c, 0x22, 0x4a, 0x16, 0x07]);
        reference::all::<u128>(&0x57164a225c19057b07164a226cbbaa8c, &[0x8c, 0xaa, 0xbb, 0x6c, 0x22, 0x4a, 0x16, 0x07, 0x7b, 0x05, 0x19, 0x5c, 0x22, 0x4a, 0x16, 0x57]);
    }
}
