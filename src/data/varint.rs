// Dependencies

use crate::bytewise::Readable;
use crate::bytewise::Reader;
use crate::bytewise::Size;
use crate::bytewise::Writable;
use crate::bytewise::Writer;

// Structs

#[derive(Debug, PartialEq)]
pub struct Varint(pub u32);

// Implementations

impl Readable for Varint {
    const SIZE: Size = Size::Variable;

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
    const SIZE: Size = Size::Variable;

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
