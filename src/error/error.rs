// Dependencies

use std::convert::Into;
use std::vec::Vec;

// Traits

pub trait Error {
    fn description(&self) -> &String;
    fn add<Text: Into<String>>(self, context: Text) -> Self;
    fn more(&self) -> &Vec<String>;
}
