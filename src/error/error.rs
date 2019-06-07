// Dependencies

use std::convert::Into;
use std::vec::Vec;
use super::attachment::Attachment;

// Traits

pub trait Error {
    fn description(&self) -> &String;
    fn add<Text: Into<String>>(self, context: Text) -> Self;
    fn attach<Payload: Attachment>(self, attachment: Payload) -> Self;
    fn more(&self) -> &Vec<String>;
    fn attachments(&self) -> &Vec<Box<dyn Attachment>>;
}
