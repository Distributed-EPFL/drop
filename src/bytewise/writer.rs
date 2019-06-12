// Dependencies

use super::errors::WriteError;
use super::errors::WriterError;
use super::writable::Writable;

// Traits

pub trait Writer : Sized {
    fn pop(&mut self, size: usize) -> Result<&[u8], WriterError>;

    fn visit<Acceptor: Writable>(&mut self, acceptor: &mut Acceptor) -> Result<(), WriteError> {
        acceptor.accept(self)
    }
}
