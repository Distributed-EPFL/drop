// Dependencies

use super::load::Load;
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

        impl<Item: Load> Load for [Item; $size] {
            default fn load<From: Writer>(from: &mut From) -> Result<Self, From::Error> {
                unsafe {
                    let mut array: [Item; $size] = std::mem::uninitialized();

                    for index in 0..$size {
                        match Item::load(from) {
                            Ok(item) => std::ptr::write(&mut array[index], item),
                            Err(err) => {
                                for item in &mut array[0..index] {
                                    std::ptr::drop_in_place(item);
                                }

                                std::mem::forget(array);
                                return Err(err);
                            }
                        }
                    }

                    return Ok(array);
                }
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

        impl Load for [u8; $size] {
            fn load<From: Writer>(from: &mut From) -> Result<Self, From::Error> {
                let mut array: [u8; $size] = [0; $size];
                from.visit(&mut array)?;
                Ok(array)
            }
        }
    )*);
}

implement!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
           25, 26, 27, 28, 29, 30, 31, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192);
