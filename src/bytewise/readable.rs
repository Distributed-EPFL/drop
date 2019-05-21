// Dependencies

use super::reader::Reader;
use super::root::Root;
use super::size::Size;

// Traits

pub trait Readable {
    const SIZE: Size;
    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error>;
}

// Implementations

impl<Value:Root> Readable for Value {
    const SIZE: Size = Value::SIZE;

    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
        visitor.read(self)
    }
}
