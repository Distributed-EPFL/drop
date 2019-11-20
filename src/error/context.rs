// Dependencies

use std::any::Any;

use super::error::Error;
use super::spotting::Spotting;
use crate::lang::Typename;

// Traits

pub trait Context {
    fn spot(self, spotting: Spotting) -> Self;
    fn add<Text: Into<String>>(self, context: Text) -> Self;
    fn attach<Payload: Any + Typename>(self, attachment: Payload) -> Self;
}

// Implementations

impl<Ok, Err: Error> Context for Result<Ok, Err> {
    fn spot(self, spotting: Spotting) -> Self {
        self.map_err(|err| err.spot(spotting))
    }

    fn add<Text: Into<String>>(self, context: Text) -> Self {
        self.map_err(|err| err.add(context))
    }

    fn attach<Payload: Any + Typename>(self, attachment: Payload) -> Self {
        self.map_err(|err| err.attach(attachment))
    }
}
