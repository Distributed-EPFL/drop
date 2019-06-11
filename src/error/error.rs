// Dependencies

use backtrace::Backtrace;
use std::convert::Into;
use std::vec::Vec;
use super::attachment::Attachment;
use super::spotting::Spotting;

// Traits

pub trait Error {
    fn description(&self) -> &String;
    fn backtrace(&self) -> &Backtrace;
    fn spot(self, spotting: Spotting) -> Self;
    fn add<Text: Into<String>>(self, context: Text) -> Self;
    fn attach<Payload: Attachment>(self, attachment: Payload) -> Self;
    fn spottings(&self) -> &Vec<Spotting>;
    fn more(&self) -> &Vec<String>;
    fn attachments(&self) -> &Vec<Box<dyn Attachment>>;
}
