// Modules

mod convert;
mod errors;
mod format;
mod hash;
mod key;
mod ops;
mod parse;
mod stream;

// Structs

pub use hash::Digest;
pub use hash::State;
pub use key::Key;
pub use stream::RxStream;
pub use stream::TxStream;

// Functions

pub use hash::hash;
pub use hash::authenticate;

// Errors

pub use errors::MalformedHex;
pub use errors::ParseHexError;
pub use errors::UnexpectedSize;
