// Dependencies

use failure::Error;
use super::readable::Readable;
use super::reader::Reader;

// Structs

pub struct Tape(usize);

// Implementations

impl Tape {
    pub fn new() -> Self {
        Tape(0)
    }

    pub fn size(&self) -> usize {
        self.0
    }
}

impl Reader for Tape {
    fn push(&mut self, chunk: &[u8]) -> Result<(), Error> {
        self.0 += chunk.len();
        Ok(())
    }

    fn visit<Acceptor: Readable>(&mut self, acceptor: &Acceptor) -> Result<(), Error> {
        if Acceptor::SIZE.is_fixed() {
            self.0 += Acceptor::SIZE.size();
            Ok(())
        } else {
            acceptor.accept(self)
        }
    }
}
