/// Cryptographic primitives using sodiumoxide
pub mod crypto;

pub mod data;

/// Async and synchronous network utilities
#[cfg(feature = "net")]
pub mod net;

/// System management code
#[cfg(feature = "system")]
pub mod system;

#[cfg(any(test, feature = "test"))]
/// Test utilities that are used all across the framework
pub mod test;

/// Re-export `async_trait` to use in implementing custom user types
#[cfg(feature = "net")]
pub use async_trait::async_trait;
