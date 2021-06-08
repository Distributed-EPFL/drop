use std::fmt;
use std::hash::{Hash, Hasher};

use super::BincodeError;

use bincode::serialize_into;

use ed25519_dalek::{
    Keypair, PublicKey as DalekPublicKey, SecretKey as DalekPrivateKey,
    Signature as DalekSignature, Signer as _, Verifier as _,
};

pub use ed25519_dalek::{
    KEYPAIR_LENGTH as KEYPAIRBYTES, PUBLIC_KEY_LENGTH as PUBLICKEYBYTES,
    SECRET_KEY_LENGTH as PRIVATEKEYBYTES,
};

use rand::rngs::OsRng;

use serde::{Deserialize, Serialize};

use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
/// Error encountered when attempting to sign data using [`PrivateKey`]
/// or [`KeyPair`]
///
/// [`PrivateKey`]: self::PrivateKey
/// [`KeyPair`]: self::KeyPair
pub enum SignError {
    #[snafu(display("failed to sign data: {}", source))]
    /// The data could not be serialized for signing
    SignSerialize {
        /// Serializer error
        source: BincodeError,
    },
}

#[derive(Debug, Snafu)]
/// Signature verification error
pub enum VerifyError {
    #[snafu(display("failed to verify data: {}", source))]
    /// The data could not be serialized
    VerifySerialize {
        /// Serializer error
        source: BincodeError,
    },

    #[snafu(display("signature verification error: {}", source))]
    /// The signature was invalid
    Dalek {
        /// Error backtrace
        source: ed25519_dalek::SignatureError,
    },
}

/// A `PublicKey` used for verifying messages
#[derive(Copy, Clone, Eq, Debug, Serialize, Deserialize)]
pub struct PublicKey(DalekPublicKey);

impl PublicKey {
    /// Get this `PublicKey` as a slice of bytes
    pub fn to_bytes(self) -> [u8; PUBLICKEYBYTES] {
        self.0.to_bytes()
    }

    /// Get a reference to the slice of byte of this `PublicKey`
    pub fn as_bytes(&self) -> &[u8; PUBLICKEYBYTES] {
        self.0.as_bytes()
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.to_bytes() == other.to_bytes()
    }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for b in self.0.as_ref() {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl Hash for PublicKey {
    fn hash<H: Hasher>(&self, h: &mut H) {
        h.write(&self.0.to_bytes())
    }
}

impl From<DalekPublicKey> for PublicKey {
    fn from(key: DalekPublicKey) -> Self {
        Self(key)
    }
}

/// A secret key used for signing messages
#[derive(Debug, Serialize, Deserialize)]
pub struct PrivateKey(DalekPrivateKey);

impl PrivateKey {
    /// Create a new `PrivateKey` containing the given bytes if they represent a valid
    /// key
    pub fn new(bytes: [u8; PRIVATEKEYBYTES]) -> Result<Self, VerifyError> {
        Ok(Self(DalekPrivateKey::from_bytes(&bytes).context(Dalek)?))
    }

    /// Get the content of this `PrivateKey` as a slice of bytes
    pub fn to_bytes(&self) -> [u8; PRIVATEKEYBYTES] {
        self.0.to_bytes()
    }

    /// Get the reference to the slice of byte of this `PrivateKey`
    pub fn as_bytes(&self) -> &[u8; PRIVATEKEYBYTES] {
        self.0.as_bytes()
    }
}

impl Eq for PrivateKey {}

impl PartialEq for PrivateKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bytes() == other.0.to_bytes()
    }
}

impl Clone for PrivateKey {
    fn clone(&self) -> Self {
        Self(DalekPrivateKey::from_bytes(&self.0.to_bytes()).unwrap())
    }
}

impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for b in self.0.as_ref() {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl From<DalekPrivateKey> for PrivateKey {
    fn from(key: DalekPrivateKey) -> Self {
        Self(key)
    }
}

/// A key pair that can be used for both signing and verifying messages
#[derive(Debug, Serialize, Deserialize)]
pub struct KeyPair(Keypair);

impl KeyPair {
    /// Create a `KeyPair` using both a random secret and public key
    pub fn random() -> Self {
        Self(Keypair::generate(&mut OsRng))
    }

    /// Get the `PublicKey` in this `KeyPair`
    pub fn public(&self) -> PublicKey {
        self.0.public.into()
    }

    /// Get the `PrivateKey` in this `KeyPair`
    pub fn private(&self) -> PrivateKey {
        PrivateKey::new(self.0.secret.to_bytes()).unwrap()
    }

    /// Get this `KeyPair` as a slice of bytes
    pub fn to_bytes(&self) -> [u8; KEYPAIRBYTES] {
        self.0.to_bytes()
    }

    /// Sign a message using this `KeyPair`
    pub fn sign<T: Serialize>(
        &self,
        message: &T,
    ) -> Result<Signature, SignError> {
        let mut buffer = Vec::new();

        serialize_into(&mut buffer, message).context(SignSerialize)?;

        Ok(self.0.sign(&buffer).into())
    }
}

impl Clone for KeyPair {
    fn clone(&self) -> Self {
        Self(Keypair::from_bytes(&self.0.to_bytes()).unwrap())
    }
}

impl PartialEq for KeyPair {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bytes() == other.0.to_bytes()
    }
}

impl Eq for KeyPair {}

/// A signature that can be used to verify the authenticity of a message
#[derive(Serialize, Deserialize, Debug)]
pub struct Signature(DalekSignature);

impl Signature {
    /// Verify that this `Signature` is valid for the given message
    pub fn verify<T: Serialize>(
        &self,
        message: &T,
        pkey: &PublicKey,
    ) -> Result<(), VerifyError> {
        let mut buffer = Vec::new();

        serialize_into(&mut buffer, message).context(VerifySerialize)?;

        pkey.0.verify(&buffer, &self.0).context(Dalek)
    }
}

impl Clone for Signature {
    fn clone(&self) -> Self {
        Self(DalekSignature::new(self.0.to_bytes()))
    }
}

impl Eq for Signature {}

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bytes() == other.0.to_bytes()
    }
}

impl From<DalekSignature> for Signature {
    fn from(signature: DalekSignature) -> Self {
        Self(signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! sign_and_verify {
        ($value:expr) => {
            let value = $value;
            let keypair = KeyPair::random();
            let pubkey = keypair.public();
            let signature = keypair.sign(&value).expect("failed to sign data");

            signature
                .verify(&value, &pubkey)
                .expect("failed to verify correct signature");
        };
    }

    #[test]
    fn sign_and_verify_u64() {
        sign_and_verify!(rand::random::<u64>());
    }

    #[test]
    fn sign_and_verify_slice_u8() {
        sign_and_verify!(rand::random::<[u8; 16]>());
    }

    #[test]
    fn sign_and_verify_struct() {
        #[derive(Serialize)]
        struct T {
            a: u8,
            b: u16,
            c: u32,
        }
        let t = T {
            a: rand::random(),
            b: rand::random(),
            c: rand::random(),
        };

        sign_and_verify!(t);
    }

    #[test]
    fn sign_and_verify_tuple() {
        sign_and_verify!(rand::random::<(u8, u16, u32, u64)>());
    }

    #[test]
    fn sign_and_verify_string() {
        sign_and_verify!("Hello World!".to_string());
    }

    #[test]
    fn bad_data() {
        let keypair = KeyPair::random();
        let signature = keypair.sign(&0u64).expect("failed to sign data");

        signature
            .verify(&1u64, &keypair.public())
            .expect_err("verified signature for wrong data");
    }

    #[test]
    fn serialize() {
        macro_rules! ser_de {
            ($($value:expr, $tp:ty), *) => ($(
                let mut buffer = Vec::new();
                let value = ($value);

                serialize_into(&mut buffer, &value).expect("serialize failed");

                let output: $tp = bincode::deserialize_from(Cursor::new(buffer)).expect("deserialize failed");

                assert_eq!(output, value, "different value");
           )*)
        }

        use std::io::Cursor;

        let keypair = KeyPair::random();

        ser_de!(
            KeyPair::random(),
            KeyPair,
            keypair.public(),
            PublicKey,
            keypair.private(),
            PrivateKey
        );
    }
}
