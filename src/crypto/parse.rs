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

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::*;

    // Test cases
    #[test]
    fn reference() {
        assert_eq!(ParseHex::<[u8; 8]>::parse_hex("0000000000000000").unwrap(), [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(ParseHex::<[u8; 8]>::parse_hex("0123456789abcdef").unwrap(), [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]);
    }

    #[test]
    fn errors() {
        ParseHex::<[u8; 8]>::parse_hex("0123456789abcde").unwrap_err();
        ParseHex::<[u8; 8]>::parse_hex("01234567q9abcdef").unwrap_err();
    }

    #[test]
    fn from() {
        assert_eq!(Digest::try_from("0000000000000000000000000000000000000000000000000000000000000000").unwrap(), Digest([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
        assert_eq!(Digest::try_from("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap(), Digest([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]));

        assert_eq!(Key::try_from("0000000000000000000000000000000000000000000000000000000000000000").unwrap(), Key([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
        assert_eq!(Key::try_from("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap(), Key([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]));
    }
}
