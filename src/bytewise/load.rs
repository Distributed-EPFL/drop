// Dependencies

use super::writable::Writable;
use super::writer::Writer;

// Traits

pub trait Load : Writable {
    fn load<From: Writer>(from: &mut From) -> Result<Self, From::Error>;
}
