// Dependencies

use super::infallible::Infallible;
use super::readable::Readable;
use super::root::Root;
use super::reader::Reader;
use super::sink::Sink;
use super::size::Size;

// Structs

pub struct Tape {
    size: usize
}

struct RootTape {
    size: usize
}

// Implementations

impl Tape {
    pub fn new() -> Tape {
        Tape{size: 0}
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl RootTape {
    fn new() -> RootTape {
        RootTape{size: 0}
    }

    fn size(&self) -> usize {
        self.size
    }
}

impl Reader for Tape {
    type Error = Infallible;

    fn visit<Acceptor: Readable>(&mut self, acceptor: &Acceptor) -> Result<(), Self::Error> {
        if let Size::Fixed(size) = Acceptor::SIZE {
            self.size += size;
        } else {
            acceptor.accept(self).unwrap();
        }

        Ok(())
    }

    fn read<Value: Root>(&mut self, value: &Value) -> Result<(), Self::Error> {
        self.size += if let Size::Fixed(size) = Value::SIZE {
            size
        } else {
            let mut root_tape = RootTape::new();
            root_tape.read(value).unwrap();
            root_tape.size()
        };

        Ok(())
    }
}

impl Sink for RootTape {
    type Error = Infallible;

    fn push(&mut self, chunk: &[u8]) -> Result<(), Self::Error> {
        self.size += chunk.len();
        Ok(())
    }
}

// Tests
// #[kcov(exclude)]

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::source::Source;

    // Structs

    struct Mother;
    struct Daughter;
    struct WeirdRoot;

    // Implementations

    impl Readable for Mother {
        const SIZE: Size = Size::Variable;

        fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
            for _ in 0..6 {
                visitor.visit(&Daughter)?;
            }

            Ok(())
        }
    }

    impl Readable for Daughter {
        const SIZE: Size = Size::Variable;

        fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error> {
            for _ in 0..3 {
                visitor.visit(&99u64)?;
            }

            for _ in 0..3 {
                visitor.visit(&WeirdRoot)?;
            }

            Ok(())
        }
    }

    impl Root for WeirdRoot {
        const SIZE: Size = Size::Variable;

        fn dump<To: Sink>(&self, to: &mut To) -> Result<(), To::Error> {
            to.push(&[0, 1, 2, 3][..])
        }

        fn load<From: Source>(_from: &mut From) -> Result<Self, From::Error> {
            Ok(WeirdRoot)
        }
    }

    // Test cases

    #[test]
    fn tape() {
        {
            let mut tape = Tape::new();
            tape.visit(&66u32).unwrap();
            assert_eq!(tape.size(), 4);
        }

        {
            let mut tape = Tape::new();
            66u32.accept(&mut tape).unwrap();
            assert_eq!(tape.size(), 4);
        }

        {
            let mut tape = Tape::new();
            Mother.accept(&mut tape).unwrap();
            assert_eq!(tape.size(), 216);
        }
    }
}
