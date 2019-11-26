use std::any::Any;
use std::convert::Into;
use std::vec::Vec;

use super::spotting::Spotting;

use backtrace::Backtrace;

/// Generic error trait
pub trait Error {
    fn description(&self) -> &String;
    fn backtrace(&self) -> &Backtrace;
    fn spot(self, spotting: Spotting) -> Self;
    fn add<Text: Into<String>>(self, context: Text) -> Self;
    fn attach<Payload: Any>(self, attachment: Payload) -> Self;
    fn spottings(&self) -> &Vec<Spotting>;
    fn more(&self) -> &Vec<String>;
    fn attachments(&self) -> &Vec<(&'static str, Box<dyn Any>)>;
}
