// Dependencies

use super::errors::ReadError;
use super::readable::Readable;
use super::reader::Reader;
use super::tape::Tape;

// Traits

pub trait Measurable {
    fn size(&self) -> Result<usize, ReadError>;
}

// Implementations

impl<Acceptor: Readable> Measurable for Acceptor {
    fn size(&self) -> Result<usize, ReadError> {
        let mut tape = Tape::new();
        tape.visit(self)?;
        Ok(tape.size())
    }
}

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::*;

    #[test]
    fn measurable() {
        assert_eq!(4u32.size().unwrap(), 4);
        assert_eq!("Hello World!".to_string().size().unwrap(), 13);
    }
}
