// Dependencies

use std::any::Any;

// Traits

pub trait Attachment : Any {
    fn typename(&self) -> &'static str;
}
