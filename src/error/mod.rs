mod context;
mod spotting;

use std::any::{type_name, Any};
use std::convert::Into;
use std::ops::Deref;
use std::vec::Vec;

pub use context::Context;

use backtrace::Backtrace;

pub use macros::error;

pub use self::spotting::Spotting;

/// Generic error trait
pub trait Error {
    fn description(&self) -> &String;
    fn backtrace(&self) -> &Backtrace;
    fn spot(self, spotting: Spotting) -> Self;
    fn comment<T: Into<String>>(self, context: T) -> Self;
    fn attach<Payload: Any>(self, attachment: Payload) -> Self;
    fn spottings(&self) -> &Vec<Spotting>;
    fn details(&self) -> &[String];
    fn attachments(&self) -> &[Attachment];
}

/// An object attached to an `Error`. This contains both the attached value
/// as well as its pretty printable name.
pub struct Attachment {
    value: Box<dyn Any>,
    typename: &'static str,
}

impl Attachment {
    pub fn new<T: Sized + 'static>(value: T) -> Self {
        Self {
            value: Box::new(value),
            typename: type_name::<T>(),
        }
    }

    /// Get a reference to the pretty printable type of this attachment.
    /// This uses the full path of the type, e.g. Vec<T> will be printed as
    /// alloc::vec::Vec<T>
    pub fn typename(&self) -> &'static str {
        &self.typename
    }

    /// Downcast the reference to a concret type
    pub fn downcast_ref<T: 'static>(&self) -> Option<&T> {
        self.value.downcast_ref::<T>()
    }
}

impl Deref for Attachment {
    type Target = dyn Any;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}
