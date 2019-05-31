// Dependencies

use failure::Error;
use super::readable::Readable;

// Traits

pub trait Reader : Sized {
    fn push(&mut self, chunk: &[u8]) -> Result<(), Error>;

    fn visit<Acceptor: Readable>(&mut self, acceptor: &Acceptor) -> Result<(), Error> {
        acceptor.accept(self)
    }
}
