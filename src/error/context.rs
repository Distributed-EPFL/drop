use std::any::Any;

use super::spotting::Spotting;
use super::Error;

pub trait Context {
    fn spot(self, spotting: Spotting) -> Self;
    fn comment<Text: Into<String>>(self, context: Text) -> Self;
    fn attach<Payload: Any>(self, attachment: Payload) -> Self;
}

impl<Ok, Err: Error> Context for Result<Ok, Err> {
    fn spot(self, spotting: Spotting) -> Self {
        self.map_err(|err| err.spot(spotting))
    }

    fn comment<Text: Into<String>>(self, context: Text) -> Self {
        self.map_err(|err| err.comment(context))
    }

    fn attach<Payload: Any>(self, attachment: Payload) -> Self {
        self.map_err(|err| {
            err.attach(crate::error::Attachment::new(attachment))
        })
    }
}
