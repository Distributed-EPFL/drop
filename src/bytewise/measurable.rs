// Dependencies

use super::readable::Readable;
use super::reader::Reader;
use super::tape::Tape;

// Traits

pub trait Measurable {
    fn size(&self) -> usize;
}

// Implementations

impl<Acceptor: Readable> Measurable for Acceptor {
    fn size(&self) -> usize {
        let mut tape = Tape::new();
        tape.visit(self).unwrap();
        tape.size()
    }
}

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::*;

    #[test]
    fn measurable() {
        assert_eq!(4u32.size(), 4);
        assert_eq!("Hello World!".to_string().size(), 13);
    }
}
