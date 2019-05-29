// Dependencies

use std::convert::TryInto;
use super::load::Load;
use super::readable::Readable;
use super::reader::Reader;
use super::size::Size;
use super::writable::Writable;
use super::writer::Writer;

// Implementations

impl Readable for bool {
    const SIZE: Size = Size::fixed(1);

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
        visitor.push(&[*self as u8])
    }
}

impl Writable for bool {
    const SIZE: Size = Size::fixed(1);

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
        *self = Self::load(visitor)?;
        Ok(())
    }
}

impl Load for bool {
    fn load<From: Writer>(from: &mut From) -> Result<Self, From::Error> {
        Ok(from.pop(1)?[0] != 0)
    }
}

macro_rules! implement {
    ($($type:ty: $size:expr), *) => ($(
        impl Readable for $type {
            const SIZE: Size = Size::fixed($size);

            fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
                visitor.push(&self.to_le_bytes())
            }
        }

        impl Writable for $type {
            const SIZE: Size = Size::fixed($size);

            fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
                *self = Self::load(visitor)?;
                Ok(())
            }
        }

        impl Load for $type {
            fn load<From: Writer>(from: &mut From) -> Result<Self, From::Error> {
                Ok(Self::from_le_bytes(from.pop($size)?.try_into().unwrap()))
            }
        }
    )*);
}

implement!(i8: 1, i16: 2, i32: 4, i64: 8, i128: 16, u8: 1, u16: 2, u32: 4, u64: 8, u128: 16);

// Tests
// #[kcov(exclude)]

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::load::Load;

    // Structs

    struct Reference(&'static [u8]);

    #[derive(Debug)]
    struct Mismatch;

    // Implementations

    impl Reader for Reference {
        type Error = Mismatch;

        fn push(&mut self, chunk: &[u8]) -> Result<(), Self::Error> {
            if self.0 == chunk { Ok(()) } else { Err(Mismatch) }
        }
    }

    impl Writer for Reference {
        type Error = Mismatch;

        fn pop(&mut self, size: usize) -> Result<&[u8], Self::Error> {
            if size == self.0.len() { Ok(self.0) } else { Err(Mismatch) }
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
