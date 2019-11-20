// Dependencies

use crate::bytewise::Readable;
use crate::bytewise::Reader;
use crate::bytewise::Size;
use crate::bytewise::Writable;
use crate::bytewise::Writer;

// Structs

#[derive(Debug, Default, PartialEq)]
pub struct Varint(pub u32);

// Implementations

impl Readable for Varint {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
        assert!(self.0 <= 0x3fffffff);

        if self.0 < 128 {
            visitor.push(&[self.0 as u8])
        } else if self.0 < 16384 {
            visitor.push(&[(self.0 >> 8) as u8 | 0x80, self.0 as u8])
        } else {
            visitor.push(&[(self.0 >> 24) as u8 | 0xc0, (self.0 >> 16) as u8, (self.0 >> 8) as u8, self.0 as u8])
        }
    }
}

impl Writable for Varint {
    const SIZE: Size = Size::variable();

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
        let alpha = visitor.pop(1)?[0];

        if alpha & 0x80 != 0 {
            if alpha & 0x40 != 0 {
                let more = visitor.pop(3)?;
                let (beta, gamma, delta) = (more[0], more[1], more[2]);

                *self = Varint(((alpha & 0x3f) as u32) << 24 | (beta as u32) << 16 | (gamma as u32) << 8 | (delta as u32));
                Ok(())
            } else {
                let more = visitor.pop(1)?;
                let beta = more[0];

                *self = Varint(((alpha & 0x7f) as u32) << 8 | (beta as u32));
                Ok(())
            }
        } else {
            *self = Varint(alpha as u32);
            Ok(())
        }
    }
}

// Tests
// #[kcov(exclude)]

#[cfg(test)]
mod tests {
    use super::*;

    // Structs

    struct Reference(&'static [u8]);

    #[derive(Debug)]
    struct Mismatch;

    // Implementations

    impl Reader for Reference {
        type Error = Mismatch;

        fn push(&mut self, chunk: &[u8]) -> Result<(), Self::Error> {
            if &self.0[0..chunk.len()] == chunk {
                *self = Reference(&self.0[chunk.len()..]);
                Ok(())
            } else { Err(Mismatch) }
        }
    }

    impl Writer for Reference {
        type Error = Mismatch;

        fn pop(&mut self, size: usize) -> Result<&[u8], Self::Error> {
            if size <= self.0.len() {
                let chunk = &self.0[0..size];
                *self = Reference(&self.0[size..]);
                Ok(chunk)
            } else { Err(Mismatch) }
        }
    }

    // Macros

    macro_rules! testcase {
        ($value:expr, $reference:expr) => {
            let value = Varint($value);
            Readable::accept(&value, &mut Reference(&$reference[..])).unwrap();

            let mut value: Varint = Default::default();

            Writable::accept(&mut value, &mut Reference(&$reference[..])).unwrap();
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
        let _ = Readable::accept(&value, &mut Reference(&[]));
    }
}
