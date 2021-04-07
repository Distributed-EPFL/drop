use std::convert::TryFrom;
use std::str::FromStr;

use super::hash::Digest;
use super::key::exchange;
use super::key::Key;
use super::sign;

use snafu::{ensure, Backtrace, OptionExt, Snafu};

use sodiumoxide::crypto::generichash::DIGEST_MIN;
use sodiumoxide::crypto::kx::{PUBLICKEYBYTES, SESSIONKEYBYTES};

#[derive(Debug, Snafu)]
pub enum ParseHexError {
    #[snafu(display("Unexpected argument size"))]
    UnexpectedSize { backtrace: Backtrace },

    #[snafu(display("malformed hexadecimal value"))]
    MalformedHex { backtrace: Backtrace },
}

trait ParseHex {
    fn parse_hex(&self) -> Result<Vec<u8>, ParseHexError>;
}

impl ParseHex for &str {
    fn parse_hex(&self) -> Result<Vec<u8>, ParseHexError> {
        let mut parsed = Vec::new();

        ensure!(self.len() % 2 == 0, UnexpectedSize);

        for index in (0..self.len()).step_by(2) {
            let value = u8::from_str_radix(&self[index..(index + 2)], 16)
                .map_err(|_| MalformedHex.build())?;

            parsed.push(value);
        }

        debug_assert_eq!(parsed.len(), self.len() / 2, "wrong output size");

        Ok(parsed)
    }
}

macro_rules! implement_from_str {
    ($($name:ty, $size:expr), *) => ($(
        impl FromStr for $name {
            type Err = ParseHexError;

            fn from_str(hex: &str) -> Result<Self, Self::Err> {
                let bytes = hex.parse_hex()?;

                ensure!(bytes.len() == $size, UnexpectedSize);

                let mut correct = [0u8; $size];

                correct.copy_from_slice(&bytes[..$size]);

                Ok(Self(correct))
            }
        }

        impl TryFrom<&str> for $name {
            type Error = ParseHexError;

            fn try_from(v: &str) -> Result<Self, Self::Error> {
                Self::from_str(v)
            }
        }
    )*)
}

implement_from_str!(Digest, DIGEST_MIN * 2, Key, SESSIONKEYBYTES);

impl FromStr for exchange::PublicKey {
    type Err = ParseHexError;

    fn from_str(hex: &str) -> Result<Self, Self::Err> {
        use sodiumoxide::crypto::kx::PublicKey;

        ensure!(hex.len() == PUBLICKEYBYTES, UnexpectedSize);

        let bytes = hex.parse_hex()?;
        let sodium =
            PublicKey::from_slice(bytes.as_slice()).context(UnexpectedSize)?;

        Ok(Self::from(sodium))
    }
}

impl FromStr for sign::PublicKey {
    type Err = ParseHexError;

    fn from_str(hex: &str) -> Result<Self, Self::Err> {
        use sodiumoxide::crypto::sign::{PublicKey, PUBLICKEYBYTES};

        ensure!(hex.len() == PUBLICKEYBYTES, UnexpectedSize);

        let slice = hex.parse_hex()?;

        let key = PublicKey::from_slice(&slice[..PUBLICKEYBYTES])
            .context(UnexpectedSize)?;

        Ok(key.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test cases
    #[test]
    fn reference() {
        assert_eq!(
            ParseHex::parse_hex(&"0000000000000000").unwrap(),
            [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]
        );
        assert_eq!(
            ParseHex::parse_hex(&"0123456789abcdef").unwrap(),
            [0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]
        );
    }

    #[test]
    fn errors() {
        ParseHex::parse_hex(&"0123456789abcde").unwrap_err();
        ParseHex::parse_hex(&"01234567q9abcdef").unwrap_err();
    }

    #[test]
    fn from() {
        assert_eq!(Digest::from_str("0000000000000000000000000000000000000000000000000000000000000000").unwrap(), Digest([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
        assert_eq!(Digest::from_str("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap(), Digest([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]));

        assert_eq!(Key::from_str("0000000000000000000000000000000000000000000000000000000000000000").unwrap(), Key([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
        assert_eq!(Key::from_str("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap(), Key([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]));
    }
}
