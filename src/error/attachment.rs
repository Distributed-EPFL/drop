// Dependencies

use crate::traits::Typename;
use std::any::Any;

// Traits

pub trait Attachment : Any + Typename {}

// Implementations

impl<Payload: Any + Typename> Attachment for Payload {}
