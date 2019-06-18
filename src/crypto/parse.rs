// Dependencies

use std::convert::TryFrom;
use super::errors::MalformedHex;
use super::errors::ParseHexError;
use super::errors::UnexpectedSize;
use super::hash::Digest;
use super::key::Key;

// Traits

pub trait ParseHex<To> {
    fn parse_hex(&self) -> Result<To, ParseHexError>;
}

// Implementations

macro_rules! implement {
    ($($size:expr), *) => ($(
        impl ParseHex<[u8; $size]> for str {
            fn parse_hex(&self) -> Result<[u8; $size], ParseHexError> {
                if self.len() != (2 * $size) { Err(UnexpectedSize::new().into()) } else {
                    let mut parsed = [u8::default(); $size];
                    for index in 0..$size {
                        parsed[index] = u8::from_str_radix(&self[(2 * index)..(2 * (index + 1))], 16)
                                            .map_err(|_| ParseHexError::from(MalformedHex::new()))?;
                    }

                    Ok(parsed)
                }
            }
        }
    )*);
}

implement!(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
           25, 26, 27, 28, 29, 30, 31, 32, 64, 128, 256, 512, 1024, 2048, 4096, 8192);

impl TryFrom<&str> for Digest {
   type Error = ParseHexError;

   fn try_from(hex: &str) -> Result<Self, ParseHexError> {
       Ok(Digest(hex.parse_hex()?))
   }
}

impl TryFrom<&str> for Key {
    type Error = ParseHexError;

    fn try_from(hex: &str) -> Result<Self, ParseHexError> {
        Ok(Key(hex.parse_hex()?))
    }
}
