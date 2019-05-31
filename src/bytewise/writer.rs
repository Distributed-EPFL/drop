// Dependencies

use failure::Error;
use super::writable::Writable;

// Traits

pub trait Writer : Sized {
    fn pop(&mut self, size: usize) -> Result<&[u8], Error>;

    fn visit<Acceptor: Writable>(&mut self, acceptor: &mut Acceptor) -> Result<(), Error> {
        acceptor.accept(self)
    }
}
