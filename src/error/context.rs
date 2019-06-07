// Dependencies

use super::attachment::Attachment;
use super::error::Error;

// Traits

pub trait Context {
    fn add<Text: Into<String>>(self, context: Text) -> Self;
    fn attach<Payload: Attachment>(self, attachment: Payload) -> Self;
}

// Implementations

impl<Ok, Err: Error> Context for Result<Ok, Err> {
    fn add<Text: Into<String>>(self, context: Text) -> Self {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(err.add(context))
        }
    }

    fn attach<Payload: Attachment>(self, attachment: Payload) -> Self {
        match self {
            Ok(ok) => Ok(ok),
            Err(err) => Err(err.attach(attachment))
        }
    }
}
