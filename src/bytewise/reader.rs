// Dependencies

use super::readable::Readable;

// Traits

pub trait Reader : Sized {
    type Error;
    fn push(&mut self, chunk: &[u8]) -> Result<(), Self::Error>;

    fn visit<Acceptor: Readable>(&mut self, acceptor: &Acceptor) -> Result<(), Self::Error> {
        acceptor.accept(self)
    }
}
