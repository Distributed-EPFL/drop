#![deny(missing_docs)]

//! This is drop, a framework for distributed systems that aims to provide most of the low level plumbing
//! required when building any kind of distributed system.
//!
//! Drop provides the most common cryptographic primitives such as message signature and verification, encrypted
//! network streams, hashing and HMAC computation tools. All these are available in the [`crypto`] module.
//!
//! Drop also provides network utilities for secure communication and offers different methods for discovering and
//! connecting to remote peers in the [`net`] module.
//!
//! Drop also provides a convenient way to implement distributed algorithms and managing connections to a lot
//! of remote peers in the [`system`] module.
//!
//! Lastly drop provides a lot of testing utilites that makes it easier to test your application in the [`test`]
//! module.
//!
//!
//! [`crypto`]: self::crypto
//! [`net`]: self::net
//! [`system`]: self::system
//! [`test`]: self::test

/// Cryptographic primitives
pub mod crypto;

/// Syncset to efficiently synchronize two sets of values
pub mod data;

/// Asynchronous secure network utilities
#[cfg(feature = "net")]
#[cfg_attr(docsrs, doc(cfg(feature = "net")))]
pub mod net;

/// System management utilities for implementing distributed algorithms
#[cfg(feature = "system")]
#[cfg_attr(docsrs, doc(cfg(feature = "system")))]
pub mod system;

#[cfg(any(test, feature = "test"))]
#[cfg_attr(docsrs, doc(cfg(feature = "test")))]
/// Test utilities that are used all across the framework
pub mod test;

/// Re-export `async_trait` to use in implementing custom user types
#[cfg(feature = "net")]
#[cfg_attr(docsrs, doc(cfg(feature = "net")))]
pub use async_trait::async_trait;
