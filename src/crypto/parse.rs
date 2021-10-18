use hex::FromHex;
use snafu::{ResultExt, Snafu};

use super::{
    hash::{self, Digest},
    key::{self, exchange, Key},
    sign,
};

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
        use crypto_kx::PublicKey;

        let mut array = [0u8; PublicKey::BYTES];
        hex::decode_to_slice(hex, &mut array).context(MalformedHex)?;

        Ok(Self::from(PublicKey::from(array)))
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

        let dalek = SecretKey::from_bytes(&slice).context(Dalek)?;

        Ok(Self::from(dalek))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TABLE: [(&str, [u8; 32]); 2] = [
        (
            "0000000000000000000000000000000000000000000000000000000000000000",
            [
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
        ),
        (
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            [
                0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23,
                0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67,
                0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
                0xcd, 0xef,
            ],
        ),
    ];

    #[test]
    fn digest_from() {
        TABLE.iter().for_each(|(s, v)| {
            assert_eq!(Digest::from_hex(s).unwrap(), Digest::from(*v))
        });
    }

    #[test]
    fn key_from() {
        TABLE.iter().for_each(|(s, v)| {
            assert_eq!(Key::from_hex(s).unwrap(), Key::from(*v))
        });
    }
}
