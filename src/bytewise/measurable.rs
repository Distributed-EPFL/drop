// Dependencies

use super::readable::Readable;
use super::tape::Tape;

// Traits

pub trait Measurable {
    fn size(&self) -> usize;
}

// Implementations

impl<Acceptor: Readable> Measurable for Acceptor {
    fn size(&self) -> usize {
        let mut tape = Tape::new();
        self.accept(&mut tape).unwrap();
        tape.size()
    }
}

// Tests
// #[kcov(exclude)]

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::reader::Reader;
    use super::super::size::Size;

    // Structs

    struct NonTrivial;

    // Implementations

    impl Readable for NonTrivial {
        const SIZE: Size = u128::SIZE;

        fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
            visitor.visit(&0u128)
        }
    }

    // Test cases

    #[test]
    fn measurable() {
        assert_eq!(66u64.size(), 8);
        assert_eq!(NonTrivial.size(), 16);
    }
}
