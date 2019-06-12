// Dependencies

use super::errors::ReadError;
use super::errors::ReaderError;
use super::readable::Readable;

// Traits

pub trait Reader : Sized {
    fn push(&mut self, chunk: &[u8]) -> Result<(), ReaderError>;

    fn visit<Acceptor: Readable>(&mut self, acceptor: &Acceptor) -> Result<(), ReadError> {
        acceptor.accept(self)
    }
}
