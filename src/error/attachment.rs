// Dependencies

use crate::traits::Typename;
use std::any::Any;

// Traits

pub trait Attachment : Any {
    fn typename(&self) -> String;
}

// Implementations

impl<Payload: Any + Typename> Attachment for Payload {
    fn typename(&self) -> String {
        Self::typename()
    }
}
