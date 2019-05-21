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
