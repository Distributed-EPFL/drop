mod convert;
mod format;
pub mod hash;
pub mod key;
mod parse;
pub mod seal;
pub mod sign;
pub mod stream;

pub use hash::Digest;
pub use key::Key;

pub use hash::authenticate;
pub use hash::hash;

/// Type alias for serializer errors
pub type BincodeError = Box<bincode::ErrorKind>;
