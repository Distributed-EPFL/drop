// Dependencies

use super::readable::Readable;
use super::root::Root;
use super::sink::Sink;

// Traits

pub trait Reader {
    type Error;
    fn visit<Acceptor: Readable>(&mut self, acceptor: &Acceptor) -> Result<(), Self::Error>;
    fn read<Value: Root>(&mut self, value: &Value) -> Result<(), Self::Error>;
}

// Implementations

impl<Visitor: Sink> Reader for Visitor {
    type Error = Visitor::Error;

    fn visit<Acceptor: Readable>(&mut self, acceptor: &Acceptor) -> Result<(), Self::Error> {
        acceptor.accept(self)
    }

    fn read<Value: Root>(&mut self, value: &Value) -> Result<(), Self::Error> {
        value.dump(self)
    }
}

// Tests
// #[kcov(exclude)]

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::infallible::Infallible;
    use std::vec::Vec;

    // Structs

    struct Sponge {
        bytes: Vec<u8>
    }

    // Implementations

    impl Sponge {
        fn new() -> Sponge {
            Sponge{bytes: Vec::new()}
        }
    }

    impl Sink for Sponge {
        type Error = Infallible;

        fn push(&mut self, chunk: &[u8]) -> Result<(), Self::Error> {
            self.bytes.extend_from_slice(chunk);
            Ok(())
        }
    }

    // Test cases

    #[test]
    fn sink() {
        let mut sponge = Sponge::new();

        sponge.visit(&0x44u32).unwrap();
        assert_eq!(&sponge.bytes[..], &[0x44, 0x00, 0x00, 0x00][..]);

        sponge.read(&0x55u8).unwrap();
        assert_eq!(&sponge.bytes[..], &[0x44, 0x00, 0x00, 0x00, 0x55][..]);

        0x66u16.accept(&mut sponge).unwrap();
        assert_eq!(&sponge.bytes[..], &[0x44, 0x00, 0x00, 0x00, 0x55, 0x66, 0x00][..]);
    }
}
