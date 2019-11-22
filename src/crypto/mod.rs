// Modules

mod convert;
mod errors;
mod format;
pub mod hash;
pub mod key;
mod ops;
mod parse;
pub mod seal;
pub mod sign;
pub mod stream;

// Structs

pub use hash::Digest;
pub use key::Key;

// Functions

pub use hash::authenticate;
pub use hash::hash;

// Errors

pub use errors::MalformedHex;
pub use errors::ParseHexError;
pub use errors::UnexpectedSize;
