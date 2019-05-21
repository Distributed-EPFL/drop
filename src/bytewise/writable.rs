// Dependencies

use super::root::Root;
use super::size::Size;
use super::writer::Writer;

// Traits

pub trait Writable {
    const SIZE: Size;
    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Visitor::Error>;
}

// Implementations

impl<Value: Root> Writable for Value {
    const SIZE: Size = Value::SIZE;

    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
        visitor.write(self)
    }
}

// Tests
// #[kcov(exclude)]

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::infallible::Infallible;

    // Structs

    struct Marker(bool);

    // Implementations

    impl Marker {
        fn new() -> Marker {
            Marker(false)
        }
    }

    impl Writer for Marker {
        type Error = Infallible;

        fn visit<Acceptor: Writable>(&mut self, acceptor: &mut Acceptor) -> Result<(), Self::Error> {
            acceptor.accept(self)
        }

        fn write<Value: Root>(&mut self, _value: &mut Value) -> Result<(), Self::Error> {
            *self = Marker(true);
            Ok(())
        }
    }

    // Test cases

    #[test]
    fn root() {
        let mut marker = Marker::new();
        44.accept(&mut marker).unwrap();
        let Marker(success) = marker;
        assert!(success);

        let mut marker = Marker::new();
        marker.visit(&mut 44).unwrap();
        let Marker(success) = marker;
        assert!(success);
    }
}
