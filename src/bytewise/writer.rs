// Dependencies

use super::root::Root;
use super::source::Source;
use super::writable::Writable;

// Traits

pub trait Writer {
    type Error;
    fn visit<Acceptor: Writable>(&mut self, acceptor: &mut Acceptor) -> Result<(), Self::Error>;
    fn write<Value: Root>(&mut self, value: &mut Value) -> Result<(), Self::Error>;
}

// Implementations

impl<Visitor: Source> Writer for Visitor {
    type Error = Visitor::Error;

    fn visit<Acceptor: Writable>(&mut self, acceptor: &mut Acceptor) -> Result<(), Self::Error> {
        acceptor.accept(self)
    }

    fn write<Value: Root>(&mut self, value: &mut Value) -> Result<(), Self::Error> {
        *value = Value::load(self)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::infallible::Infallible;

    // Structs

    struct Sponge {
        bytes: &'static [u8],
        cursor: usize
    }

    // Implementations

    impl Sponge {
        fn new(bytes: &'static [u8]) -> Sponge {
            Sponge{bytes, cursor: 0}
        }
    }

    impl Source for Sponge {
        type Error = Infallible;

        fn pop(&mut self, size: usize) -> Result<&[u8], Self::Error> {
            let slice = &self.bytes[self.cursor..(self.cursor + size)];
            self.cursor += size;
            Ok(slice)
        }
    }

    // Test cases

    #[test]
    fn source() {
        let mut sponge = Sponge::new(&[0x44, 0x00, 0x00, 0x00, 0x55, 0x66, 0x00][..]);

        let mut value: u32 = 0;
        sponge.visit(&mut value).unwrap();
        assert_eq!(value, 0x44);

        let mut value: u8 = 0;
        sponge.write(&mut value).unwrap();
        assert_eq!(value, 0x55);

        let mut value: u16 = 0;
        value.accept(&mut sponge).unwrap();
        assert_eq!(value, 0x66);
    }
}
