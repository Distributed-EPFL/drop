use hex::FromHex;
use snafu::{Backtrace, OptionExt, ResultExt, Snafu};

use super::hash::{self, Digest};
use super::key::{self, exchange, Key};
use super::sign;

/// Error encountered when parsing hexadecimal strings using the [`ParseHex`] trait
///
/// [`ParseHex`]: self::ParseHex
#[derive(Debug, Snafu)]
pub enum ParseHexError {
    #[snafu(display("malformed hexadecimal value: {}", source))]
    /// Invalid characters were present in the string
    MalformedHex {
        /// Error backtrace
        source: hex::FromHexError,
    },

    /// Dalek error
    Dalek {
        /// Error source
        source: ed25519_dalek::SignatureError,
    },

    /// Sodium error
    Sodium {
        /// Error backtrace
        backtrace: Backtrace,
    },
}

macro_rules! from_hex_with_slice_impl {
    ($($name:ty, $size:expr), *) => ($(
        impl FromHex for $name {
            type Error = ParseHexError;

            fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
                let mut slice = [0u8; $size];
                hex::decode_to_slice(hex, &mut slice).context(MalformedHex)?;

                Ok(Self::from(slice))
            }
        }
    )*)
}

from_hex_with_slice_impl!(Digest, hash::SIZE);
from_hex_with_slice_impl!(Key, key::SIZE);

impl FromHex for exchange::PublicKey {
    type Error = ParseHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        use sodiumoxide::crypto::kx::{PublicKey, PUBLICKEYBYTES};

        let mut slice = [0u8; PUBLICKEYBYTES];
        hex::decode_to_slice(hex, &mut slice).context(MalformedHex)?;

        let key = PublicKey::from_slice(&slice).context(Sodium)?;

        Ok(Self::from(key))
    }
}

impl FromHex for exchange::PrivateKey {
    type Error = ParseHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        use sodiumoxide::crypto::kx::{SecretKey, SECRETKEYBYTES};

        let mut slice = [0u8; SECRETKEYBYTES];
        hex::decode_to_slice(hex, &mut slice).context(MalformedHex)?;

        let key = SecretKey::from_slice(&slice).context(Sodium)?;

        Ok(Self::from(key))
    }
}

impl FromHex for sign::PublicKey {
    type Error = ParseHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        use ed25519_dalek::{PublicKey, PUBLIC_KEY_LENGTH};

        let mut slice = [0u8; PUBLIC_KEY_LENGTH];
        hex::decode_to_slice(hex, &mut slice).context(MalformedHex)?;

        let key = PublicKey::from_bytes(&slice).context(Dalek)?;

        Ok(Self::from(key))
    }
}

impl FromHex for sign::PrivateKey {
    type Error = ParseHexError;

    fn from_hex<T: AsRef<[u8]>>(hex: T) -> Result<Self, Self::Error> {
        use ed25519_dalek::{SecretKey, SECRET_KEY_LENGTH};

        let mut slice = [0u8; SECRET_KEY_LENGTH];
        hex::decode_to_slice(hex, &mut slice).context(MalformedHex)?;

        let sodium = SecretKey::from_bytes(&slice).context(Dalek)?;

        Ok(Self::from(sodium))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from() {
        assert_eq!(Digest::from_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap(), Digest::from([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
        assert_eq!(Digest::from_hex("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap(), Digest::from([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]));

        assert_eq!(Key::from_hex("0000000000000000000000000000000000000000000000000000000000000000").unwrap(), Key::from([0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]));
        assert_eq!(Key::from_hex("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap(), Key::from([0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]));
    }
}
