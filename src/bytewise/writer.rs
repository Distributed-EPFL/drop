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
