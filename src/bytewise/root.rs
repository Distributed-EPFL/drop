// Dependencies

use super::sink::Sink;
use super::source::Source;

// Traits

pub trait Root : Sized {
    fn dump<To: Sink>(&self, to: &mut To) -> Result<(), To::Error>;
    fn load<From: Source>(from: &mut From) -> Result<Self, From::Error>;
}
