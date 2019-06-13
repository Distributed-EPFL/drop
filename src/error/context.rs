// Dependencies

use crate::lang::Typename;
use std::any::Any;
use super::error::Error;
use super::spotting::Spotting;

// Traits

pub trait Context {
    fn spot(self, spotting: Spotting) -> Self;
    fn add<Text: Into<String>>(self, context: Text) -> Self;
    fn attach<Payload: Any + Typename>(self, attachment: Payload) -> Self;
}

// Implementations

impl<Ok, Err: Error> Context for Result<Ok, Err> {
    fn spot(self, spotting: Spotting) -> Self {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(err.spot(spotting))
        }
    }

    fn add<Text: Into<String>>(self, context: Text) -> Self {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(err.add(context))
        }
    }

    fn attach<Payload: Any + Typename>(self, attachment: Payload) -> Self {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(err.attach(attachment))
        }
    }
}
