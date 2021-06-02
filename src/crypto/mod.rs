mod convert;
mod format;
/// Hashing and HMAC utilities
pub mod hash;

/// Cryptographic primitives for secure network exchange
pub mod key;
mod parse;

/// Utilities for sealed cryptographic boxes
pub mod seal;

/// Signature computation and verification utilities
pub mod sign;

/// Secure network stream utilities
pub mod stream;

pub mod bls;

pub use hash::Digest;
pub use key::Key;

pub use hash::authenticate;
pub use hash::hash;

pub use parse::{ParseHex, ParseHexError};

/// Type alias for serializer errors
pub type BincodeError = Box<bincode::ErrorKind>;
