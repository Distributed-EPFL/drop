// Dependencies

use super::errors::WriteError;
use super::errors::WriterError;
use super::load::Load;
use super::writer::Writer;

// Structs

pub struct Deserializer<'s>(pub &'s [u8]);

// Implementations

impl Writer for Deserializer<'_> {
    fn pop(&mut self, size: usize) -> Result<&[u8], WriterError> {
        if self.0.len() >= size {
            let slice = &self.0[0..size];
            self.0 = &self.0[size..];

            Ok(slice)
        } else {
            Err(WriterError::new("EndOfBuffer"))
        }
    }
}

// Functions

pub fn deserialize<Target: Load>(buffer: &[u8]) -> Result<Target, WriteError> {
    Target::load(&mut Deserializer(&buffer[..]))
}

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::*;

    // Test cases

    #[test]
    fn reference() {
        assert_eq!(deserialize::<[u32; 4]>(&[0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00]).unwrap(), [0x01, 0x02, 0x03, 0x04]);
        deserialize::<[u32; 4]>(&[0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00]).unwrap_err();
    }
}
