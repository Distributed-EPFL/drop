use super::errors::{SignError, SodiumError, VerifyError};

use bincode::serialize_into;

use serde::Serialize;

use sodiumoxide::crypto::sign::{
    gen_keypair, sign_detached, verify_detached, PublicKey as SodiumPublicKey,
    SecretKey as SodiumSecretKey,
};

pub use sodiumoxide::crypto::sign::{
    Signature, PUBLICKEYBYTES as PUBKEY_LENGTH,
    SECRETKEYBYTES as SECKEY_LENGTH, SIGNATUREBYTES as SIGN_LENGTH,
};

/// A public key used for verifying messages
#[derive(Clone)]
pub struct PublicKey(SodiumPublicKey);

/// A secret key used for signing messages
#[derive(Clone)]
pub struct SecretKey(SodiumSecretKey);

/// A key pair that can be used for both signing and verifying messages
#[derive(Clone)]
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

    /// Creata a new `Signer` with a randomly generated `KeyPair`
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
    pub fn private(&self) -> &SecretKey {
        &self.keypair.secret
    }

    /// Sign some serializable data using the `SecretKey` in this `Signer`
    pub fn sign<T: Serialize>(
        &mut self,
        data: &T,
    ) -> Result<Signature, SignError> {
        self.buffer.clear();
        serialize_into(&mut self.buffer, data)?;

        Ok(sign_detached(&self.buffer, &self.private().0))
    }

    /// Verify that the provided `Signature` is valid for the given message
    pub fn verify<T: Serialize>(
        &mut self,
        signature: &Signature,
        pubkey: &PublicKey,
        message: &T,
    ) -> Result<(), VerifyError> {
        self.buffer.clear();
        serialize_into(&mut self.buffer, message)?;

        if verify_detached(signature, &self.buffer, &pubkey.0) {
            Ok(())
        } else {
            Err(SodiumError::new().into())
        }
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

        signature.0.last_mut().map(|x| *x = x.wrapping_add(1));

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
