use std::hash::Hash;

use super::BincodeError;

use bincode::serialize_into;

use serde::{Deserialize, Serialize};

use snafu::{ensure, Backtrace, ResultExt, Snafu};

use sodiumoxide::crypto::sign::{
    gen_keypair, sign_detached, verify_detached, PublicKey as SodiumPublicKey,
    SecretKey as SodiumSecretKey,
};

pub use sodiumoxide::crypto::sign::{
    Signature, PUBLICKEYBYTES as PUBLIC_LENGTH,
    SECRETKEYBYTES as SECRET_LENGTH, SIGNATUREBYTES as SIGNATURE_LENGTH,
};

#[derive(Debug, Snafu)]
pub enum SignError {
    #[snafu(display("failed to sign data: {}", source))]
    SignSerialize { source: BincodeError },
}

#[derive(Debug, Snafu)]
pub enum VerifyError {
    #[snafu(display("failed to verify data: {}", source))]
    VerifySerialize { source: BincodeError },

    #[snafu(display("invalid signature"))]
    Sodium { backtrace: Backtrace },
}

/// A public key used for verifying messages
#[derive(
    Copy,
    Clone,
    Deserialize,
    Eq,
    Hash,
    PartialEq,
    PartialOrd,
    Ord,
    Serialize,
    Debug,
)]
pub struct PublicKey(SodiumPublicKey);

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// A secret key used for signing messages
#[derive(Clone, Deserialize, Eq, PartialEq, Serialize)]
pub struct SecretKey(SodiumSecretKey);

impl SecretKey {
    /// Compute the `PublicKey` associated with this `SecretKey`
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.public_key())
    }
}

impl AsRef<[u8]> for SecretKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

/// A key pair that can be used for both signing and verifying messages
#[derive(Clone, PartialEq, Eq)]
pub struct KeyPair {
    public: PublicKey,
    secret: SecretKey,
}

impl KeyPair {
    /// Create a `KeyPair` using both a random secret and public key
    pub fn random() -> Self {
        let (public, secret) = gen_keypair();
        let public = PublicKey(public);
        let secret = SecretKey(secret);

        Self { public, secret }
    }

    /// Get the `PublicKey` in this `KeyPair`
    pub fn public(&self) -> &PublicKey {
        &self.public
    }

    /// Get the `SecretKey` in this `KeyPair`
    pub fn secret(&self) -> &SecretKey {
        &self.secret
    }
}

impl From<SecretKey> for KeyPair {
    fn from(secret: SecretKey) -> Self {
        let public = secret.public_key();

        Self { secret, public }
    }
}

/// A `Signer` is used to sign data and verify signatures
pub struct Signer {
    keypair: KeyPair,
    buffer: Vec<u8>,
}

impl Signer {
    /// Create a new `Signer` that will use the given `KeyPair`
    pub fn new(keypair: KeyPair) -> Self {
        Self {
            keypair,
            buffer: Vec::new(),
        }
    }

    /// Create a new `Signer` with a randomly generated `KeyPair`
    pub fn random() -> Self {
        Self {
            keypair: KeyPair::random(),
            buffer: Vec::new(),
        }
    }

    /// Get a reference to the `PublicKey` used by this `Signer`
    pub fn public(&self) -> &PublicKey {
        &self.keypair.public
    }

    /// Get a reference to the `SecretKey` used by this `Signer`
    pub fn secret(&self) -> &SecretKey {
        &self.keypair.secret
    }

    /// Sign some serializable data using the `SecretKey` in this `Signer`
    pub fn sign<T: Serialize>(
        &mut self,
        message: &T,
    ) -> Result<Signature, SignError> {
        self.buffer.clear();
        serialize_into(&mut self.buffer, message).context(SignSerialize)?;

        Ok(sign_detached(&self.buffer, &self.secret().0))
    }

    /// Verify that the provided `Signature` is valid for the given message
    pub fn verify<T: Serialize>(
        &mut self,
        signature: &Signature,
        public: &PublicKey,
        message: &T,
    ) -> Result<(), VerifyError> {
        self.buffer.clear();
        serialize_into(&mut self.buffer, message).context(VerifySerialize)?;

        ensure!(verify_detached(signature, &self.buffer, &public.0), Sodium);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! sign_and_verify {
        ($value:expr) => {
            let value = $value;
            let keypair = KeyPair::random();
            let pubkey = keypair.public.clone();
            let mut signer = Signer::new(keypair);
            let signature = signer.sign(&value).expect("failed to sign data");

            signer
                .verify(&signature, &pubkey, &value)
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
    fn bad_signature() {
        let keypair = KeyPair::random();
        let mut signer = Signer::new(keypair.clone());

        let mut signature = signer.sign(&0u64).expect("failed to sign data");

        if let Some(x) = signature.0.last_mut() {
            *x = x.wrapping_add(1);
        }

        signer
            .verify(&signature, &keypair.public, &0u64)
            .expect_err("verified bad signature");
    }

    #[test]
    fn bad_data() {
        let keypair = KeyPair::random();
        let mut signer = Signer::new(keypair.clone());
        let signature = signer.sign(&0u64).expect("failed to sign data");

        signer
            .verify(&signature, &keypair.public, &1u64)
            .expect_err("verified signature for wrong data");
    }
}
