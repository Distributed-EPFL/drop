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
