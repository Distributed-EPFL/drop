// Dependencies

use std::convert::TryInto;
use super::sink::Sink;
use super::source::Source;

// Traits

pub trait Root : Sized {
    fn dump<To: Sink>(&self, to: &mut To) -> Result<(), To::Error>;
    fn load<From: Source>(from: &mut From) -> Result<Self, From::Error>;
}

// Implementations

impl Root for bool {
    fn dump<To: Sink>(&self, to: &mut To) -> Result<(), To::Error> {
        to.push(if *self { &[1] } else { &[0] })
    }

    fn load<From: Source>(from: &mut From) -> Result<Self, From::Error> {
        let value = from.pop(1)?[0] != 0;
        Ok(value)
    }
}

macro_rules! implement {
    ($($type:ty: $size:expr), *) => ($(
        impl Root for $type {
            fn dump<To: Sink>(&self, to: &mut To) -> Result<(), To::Error> {
                to.push(&self.to_le_bytes())
            }

            fn load<From: Source>(from: &mut From) -> Result<Self, From::Error> {
                let value = Self::from_le_bytes(from.pop($size)?.try_into().unwrap());
                Ok(value)
            }
        }
    )*);
}

implement!(i8: 1, i16: 2, i32: 4, i64: 8, i128: 16, u8: 1, u16: 2, u32: 4, u64: 8, u128: 16);
