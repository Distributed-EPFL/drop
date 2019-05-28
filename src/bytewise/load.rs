// Dependencies

use super::writable::Writable;
use super::writer::Writer;

// Traits

pub trait Load : Writable {
    fn load<From: Writer>(from: &mut From) -> Result<Self, From::Error>;
}

// Implementations

impl<Value: Writable + Default> Load for Value {
    default fn load<From: Writer>(from: &mut From) -> Result<Self, From::Error> {
        let mut value = Default::default();
        from.visit(&mut value)?;
        Ok(value)
    }
}
