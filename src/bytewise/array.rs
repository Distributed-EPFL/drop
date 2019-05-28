// Dependencies

use super::readable::Readable;
use super::reader::Reader;
use super::size::Size;
use super::writable::Writable;
use super::writer::Writer;

// Implementations

macro_rules! implement {
    ($($size:expr), *) => ($(
        impl<Item: Readable> Readable for [Item; $size] {
            const SIZE: Size = Size::mul($size, Item::SIZE);

            default fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
                for item in self.iter() {
                    visitor.visit(item)?;
                }

                Ok(())
            }
        }

        impl<Item: Writable> Writable for [Item; $size] {
            const SIZE: Size = Size::mul($size, Item::SIZE);

            default fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
                for item in self.iter_mut() {
                    visitor.visit(item)?;
                }

                Ok(())
            }
        }

        impl Readable for [u8; $size] {
            fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
                visitor.push(self)
            }
        }

        impl Writable for [u8; $size] {
            fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
                self.clone_from_slice(visitor.pop($size)?);
                Ok(())
            }
        }
    )*);
}

implement!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
           25, 26, 27, 28, 29, 30, 31, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192);
