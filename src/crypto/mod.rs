// Modules

mod convert;
mod errors;
mod format;
pub mod hash;
mod key;
mod ops;
mod parse;
pub mod stream;

// Structs

pub use hash::Digest;
pub use key::Key;

// Functions

pub use hash::hash;
pub use hash::authenticate;

// Errors

pub use errors::MalformedHex;
pub use errors::ParseHexError;
pub use errors::UnexpectedSize;
