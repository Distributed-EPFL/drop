use std::fmt;

/// A convenient macro to derive all required traits for your message types
pub use drop_derive::message;
use serde::{Deserialize, Serialize};

/// A trait bound for types that can be used as messages
pub trait Message:
    for<'de> Deserialize<'de> + Serialize + fmt::Debug + Send + Sync + Clone
{
}

impl<T> Message for T where
    T: for<'de> Deserialize<'de> + Serialize + fmt::Debug + Send + Sync + Clone
{
}
