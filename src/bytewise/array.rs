// Dependencies

use super::errors::ReadError;
use super::errors::WriteError;
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

            default fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
                for item in self.iter() {
                    visitor.visit(item)?;
                }

                Ok(())
            }
        }

        impl<Item: Writable> Writable for [Item; $size] {
            const SIZE: Size = Size::mul($size, Item::SIZE);

            default fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
                for item in self.iter_mut() {
                    visitor.visit(item)?;
                }

                Ok(())
            }
        }

        impl<Item: Load> Load for [Item; $size] {
            default fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
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
            fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), ReadError> {
                visitor.push(self)?;
                Ok(())
            }
        }

        impl Writable for [u8; $size] {
            fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError> {
                self.clone_from_slice(visitor.pop($size)?);
                Ok(())
            }
        }

        impl Load for [u8; $size] {
            fn load<From: Writer>(from: &mut From) -> Result<Self, WriteError> {
                let mut array: [u8; $size] = [0; $size];
                from.visit(&mut array)?;
                Ok(array)
            }
        }
    )*);
}

implement!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
           25, 26, 27, 28, 29, 30, 31, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192);

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::super::testing::reference;

    // Test cases

    #[test]
    fn default() {
        reference::all::<[bool; 8]>(&[true, false, true, true, false, false, true, true], &[0x01, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x01]);
        reference::all::<[u32; 4]>(&[0x10, 0x11, 0x12, 0x13], &[0x10, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00]);
        reference::all::<[[u32; 2]; 2]>(&[[0x10, 0x11], [0x12, 0x13]], &[0x10, 0x00, 0x00, 0x00, 0x11, 0x00, 0x00, 0x00, 0x12, 0x00, 0x00, 0x00, 0x13, 0x00, 0x00, 0x00])
    }

    #[test]
    fn shortcut() {
        reference::all::<[u8; 16]>(&[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f], &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f]);
        reference::all::<[[u8; 4]; 4]>(&[[0x00, 0x01, 0x02, 0x03], [0x04, 0x05, 0x06, 0x07], [0x08, 0x09, 0x0a, 0x0b], [0x0c, 0x0d, 0x0e, 0x0f]], &[0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f])
    }
}
