// Dependencies

use failure::Error;
use super::load::Load;
use super::readable::Readable;
use super::reader::Reader;
use super::size::Size;
use super::writable::Writable;
use super::writer::Writer;

// Implementations

macro_rules! size {
    ($only:ident) => { <$only>::SIZE };
    ($head:ident, $($tail:ident),+) => { Size::add(<$head>::SIZE, size!($($tail),+)) };
}

macro_rules! implement {
    ($($types:ident),+) => {
        impl<$($types: Readable),+> Readable for ($($types,)+) {
            const SIZE: Size = size!($($types),+);

            #[allow(non_snake_case)]
            fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Error> {
                let ($($types,)+) = self;
                $(visitor.visit($types)?;)+

                Ok(())
            }
        }

        impl<$($types: Writable),+> Writable for ($($types,)+) {
            const SIZE: Size = size!($($types),+);

            #[allow(non_snake_case)]
            fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), Error> {
                let ($($types,)+) = self;
                $(visitor.visit($types)?;)+

                Ok(())
            }
        }

        impl<$($types: Load),+> Load for ($($types,)+) {
            fn load<From: Writer>(from: &mut From) -> Result<Self, Error> {
                Ok(($($types::load(from)?,)+))
            }
        }
    }
}

implement!(A);
implement!(A, B);
implement!(A, B, C);
implement!(A, B, C, D);
implement!(A, B, C, D, E);
implement!(A, B, C, D, E, F);
implement!(A, B, C, D, E, F, G);
implement!(A, B, C, D, E, F, G, H);
implement!(A, B, C, D, E, F, G, H, I);
implement!(A, B, C, D, E, F, G, H, I, J);
implement!(A, B, C, D, E, F, G, H, I, J, K);
implement!(A, B, C, D, E, F, G, H, I, J, K, L);
