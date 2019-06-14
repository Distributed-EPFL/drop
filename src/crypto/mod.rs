// Modules

mod errors;
mod hash;

// Structs

pub use hash::Digest;
pub use hash::State;

// Functions

pub use hash::hash;

// Errors

pub use errors::MalformedHex;
pub use errors::ParseHexError;
pub use errors::UnexpectedSize;
