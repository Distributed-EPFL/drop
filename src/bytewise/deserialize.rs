// Dependencies

use std::vec::Vec;
use super::errors::EndOfBuffer;
use super::load::Load;
use super::writer::Writer;

// Structs

struct Deserializer<'s>(&'s [u8]);

// Implementations

impl Writer for Deserializer<'_> {
    type Error = EndOfBuffer;

    fn pop(&mut self, size: usize) -> Result<&[u8], Self::Error> {
        if self.0.len() >= size {
            let slice = &self.0[0..size];
            self.0 = &self.0[size..];

            Ok(slice)
        } else {
            Err(EndOfBuffer)
        }
    }
}

// Functions

pub fn deserialize<Target: Load>(buffer: &Vec<u8>) -> Result<Target, EndOfBuffer> {
    Target::load(&mut Deserializer(&buffer[..]))
}
