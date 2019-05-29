// Dependencies

use std::vec::Vec;
use super::error::Infallible;
use super::measurable::Measurable;
use super::readable::Readable;
use super::reader::Reader;

// Structs

struct Serializer(Vec<u8>);

// Implementations

impl Reader for Serializer {
    type Error = Infallible;

    fn push(&mut self, chunk: &[u8]) -> Result<(), Self::Error> {
        self.0.extend_from_slice(chunk);
        Ok(())
    }
}

// Functions

pub fn serialize<Acceptor: Readable>(acceptor: &Acceptor) -> Vec<u8> {
    let mut serializer = Serializer(Vec::with_capacity(acceptor.size()));
    serializer.visit(acceptor).unwrap();
    serializer.0
}
