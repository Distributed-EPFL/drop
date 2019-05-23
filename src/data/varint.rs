// Dependencies

use crate::bytewise::Base;
use crate::bytewise::Sink;
use crate::bytewise::Size;
use crate::bytewise::Source;

// Structs

#[derive(Debug, PartialEq)]
pub struct Varint(pub u32);

// Implementations

impl Base for Varint {
    const SIZE: Size = Size::Variable;

    fn dump<To: Sink>(&self, to: &mut To) -> Result<(), To::Error> {
        let Varint(value) = self;
        assert!(*value <= 0x3fffffff);

        if *value < 128 {
            to.push(&[*value as u8])
        } else if *value < 16384 {
            to.push(&[(*value >> 8) as u8 | 0x80, *value as u8])
        } else {
            to.push(&[(*value >> 24) as u8 | 0xc0, (*value >> 16) as u8, (*value >> 8) as u8, *value as u8])
        }
    }

    fn load<From: Source>(from: &mut From) -> Result<Self, From::Error> {
        let alpha = from.pop(1)?[0];

        if alpha & 0x80 != 0 {
            if alpha & 0x40 != 0 {
                let more = from.pop(3)?;
                let (beta, gamma, delta) = (more[0], more[1], more[2]);

                Ok(Varint(
                    ((alpha & 0x3f) as u32) << 24 | (beta as u32) << 16 | (gamma as u32) << 8 | (delta as u32)
                ))
            } else {
                let more = from.pop(1)?;
                let beta = more[0];

                Ok(Varint(
                    ((alpha & 0x7f) as u32) << 8 | (beta as u32)
                ))
            }
        } else {
            Ok(Varint(alpha as u32))
        }
    }
}

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
            if &reference[0..chunk.len()] == chunk {
                *self = Reference(&reference[chunk.len()..]);
                Ok(())
            } else { Err(Mismatch::Content) }
        }
    }

    impl Source for Reference {
        type Error = Mismatch;

        fn pop(&mut self, size: usize) -> Result<&[u8], Self::Error> {
            let Reference(reference) = self;
            if size <= reference.len() {
                let chunk = &reference[0..size];
                *self = Reference(&reference[size..]);
                Ok(chunk)
            } else { Err(Mismatch::Size) }
        }
    }

    // Macros

    macro_rules! testcase {
        ($value:expr, $reference:expr) => {
            let value = Varint($value);
            value.dump(&mut Reference(&$reference[..])).unwrap();

            let value = Varint::load(&mut Reference(&$reference[..])).unwrap();
            assert_eq!(value, Varint($value));
        }
    }

    // Test cases

    #[test]
    fn varint() {
        testcase!(0x07, [0x07]);
        testcase!(0x0765, [0x87, 0x65]);
        testcase!(0x078495, [0xc0, 0x07, 0x84, 0x95]);
        testcase!(0x07849583, [0xc7, 0x84, 0x95, 0x83]);
    }

    #[test]
    #[should_panic]
    fn bounds() {
        let value = Varint(0x40000000);
        let _ = value.dump(&mut Reference(&[]));
    }
}
