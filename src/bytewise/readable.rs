// Dependencies

use super::reader::Reader;
use super::size::Size;

// Traits

pub trait Readable : Sized {
    const SIZE: Size;
    fn accept<Visitor: Reader>(&self, visitor: &mut Visitor) -> Result<(), Visitor::Error>;
}
