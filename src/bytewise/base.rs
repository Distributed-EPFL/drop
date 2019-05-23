// Dependencies

use std::convert::TryInto;
use super::sink::Sink;
use super::size::Size;
use super::source::Source;

// Traits

pub trait Base : Sized {
    const SIZE: Size;
    fn dump<To: Sink>(&self, to: &mut To) -> Result<(), To::Error>;
    fn load<From: Source>(from: &mut From) -> Result<Self, From::Error>;
}

// Implementations

impl Base for bool {
    const SIZE: Size = Size::Fixed(1);

    fn dump<To: Sink>(&self, to: &mut To) -> Result<(), To::Error> {
        to.push(if *self { &[1] } else { &[0] })
    }

    fn load<From: Source>(from: &mut From) -> Result<Self, From::Error> {
        let value = from.pop(1)?[0] != 0;
        Ok(value)
    }
}

macro_rules! implement {
    ($($type:ty: $size:expr), *) => ($(
        impl Base for $type {
            const SIZE: Size = Size::Fixed($size);

            fn dump<To: Sink>(&self, to: &mut To) -> Result<(), To::Error> {
                to.push(&self.to_le_bytes())
            }

            fn load<From: Source>(from: &mut From) -> Result<Self, From::Error> {
                let value = Self::from_le_bytes(from.pop($size)?.try_into().unwrap());
                Ok(value)
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

    // Enums

    #[derive(Debug)]
    enum Mismatch {
        Content,
        Size
    }

    // Structs

    struct Reference(&'static [u8]);

    // Implementations

    impl Sink for Reference {
        type Error = Mismatch;

        fn push(&mut self, chunk: &[u8]) -> Result<(), Self::Error> {
            let Reference(reference) = self;
            if *reference == chunk { Ok(()) } else { Err(Mismatch::Content) }
        }
    }

    impl Source for Reference {
        type Error = Mismatch;

        fn pop(&mut self, size: usize) -> Result<&[u8], Self::Error> {
            let Reference(reference) = self;
            if size == reference.len() { Ok(reference) } else { Err(Mismatch::Size) }
        }
    }

    // Macros

    macro_rules! testcase {
        ($type:ty, $value:expr, $reference:expr) => {
            let value: $type = $value;
            value.dump(&mut Reference(&$reference[..])).unwrap();

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
