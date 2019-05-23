// Dependencies

use super::base::Base;
use super::reader::Reader;
use super::size::Size;

// Traits

pub trait Readable {
    const SIZE: Size;
    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error>;
}

// Implementations

impl<Value:Base> Readable for Value {
    const SIZE: Size = Value::SIZE;

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
        visitor.read(self)
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

    impl Reader for Marker {
        type Error = Infallible;

        fn visit<Acceptor: Readable>(&mut self, acceptor: &Acceptor) -> Result<(), Self::Error> {
            acceptor.accept(self)
        }

        fn read<Value: Base>(&mut self, _value: &Value) -> Result<(), Self::Error> {
            *self = Marker(true);
            Ok(())
        }
    }

    // Test cases

    #[test]
    fn base() {
        let mut marker = Marker::new();
        44.accept(&mut marker).unwrap();
        let Marker(success) = marker;
        assert!(success);

        let mut marker = Marker::new();
        marker.visit(&44).unwrap();
        let Marker(success) = marker;
        assert!(success);
    }
}
