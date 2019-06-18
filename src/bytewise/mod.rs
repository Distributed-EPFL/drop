// Modules

mod array;
mod collections;
mod deserialize;
mod errors;
mod load;
mod measurable;
mod primitive;
mod readable;
mod reader;
mod serialize;
mod size;
mod string;
mod tape;
mod tuple;
mod vec;
mod writable;
mod writer;

// Traits

pub use load::Load;
pub use measurable::Measurable;
pub use readable::Readable;
pub use reader::Reader;
pub use writable::Writable;
pub use writer::Writer;

// Enums

pub use size::Size;

// Structs

pub use deserialize::Deserializer;
pub use serialize::Serializer;

// Functions

pub use deserialize::deserialize;
pub use serialize::serialize;

// Errors

pub use errors::ReadError;
pub use errors::ReadableError;
pub use errors::ReaderError;
pub use errors::WritableError;
pub use errors::WriteError;
pub use errors::WriterError;

// Tests

#[cfg(test)]
mod testing;
