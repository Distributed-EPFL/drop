// Dependencies

use super::errors::WriteError;
use super::size::Size;
use super::writer::Writer;

// Traits

pub trait Writable : Sized {
    const SIZE: Size;
    fn accept<Visitor: Writer>(&mut self, visitor: &mut Visitor) -> Result<(), WriteError>;
}
