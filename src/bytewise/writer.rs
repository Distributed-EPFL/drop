// Dependencies

use super::writable::Writable;

// Traits

pub trait Writer : Sized {
    type Error;
    fn pop(&mut self, size: usize) -> Result<&[u8], Self::Error>;

    fn visit<Acceptor: Writable>(&mut self, acceptor: &mut Acceptor) -> Result<(), Self::Error> {
        acceptor.accept(self)
    }
}
