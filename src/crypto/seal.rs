use super::stream::{DecryptError, EncryptError};

use bincode::{
    deserialize, serialize_into, serialized_size, ErrorKind as BincodeErrorKind,
};

use serde::{Deserialize, Serialize};

use snafu::{ensure, Backtrace, ResultExt, Snafu};

use sodiumoxide::crypto::box_::{
    gen_keypair, gen_nonce, open_detached, seal_detached, Nonce as SodiumNonce,
    PublicKey as SodiumPublicKey, SecretKey as SodiumSecretKey,
    Tag as SodiumTag, MACBYTES, NONCEBYTES, PUBLICKEYBYTES, SECRETKEYBYTES,
};

/// Length of the nonce prefixing each message
pub const NONCE_LENGTH: usize = NONCEBYTES;

/// Length of asymmetric public keys used by `Seal`
pub const PUBLIC_LENGTH: usize = PUBLICKEYBYTES;

/// Length of asymmetric secret keys used by `Seal`
pub const SECRET_LENGTH: usize = SECRETKEYBYTES;

/// Size of the authentication tag used
pub const TAG_LENGTH: usize = MACBYTES;

/// Size of the header at the beginning of each message
pub const HEADER_LENGTH: usize = NONCE_LENGTH + TAG_LENGTH;

/// A public key used to exchange message using asymmetric encryption
/// through a `Seal`
#[derive(Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct PublicKey(SodiumPublicKey);

/// A secret key used to exchange message using asymmetric encryption
/// through a `Seal`
#[derive(Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct SecretKey(SodiumSecretKey);

/// An asymmetric key pair
#[derive(Clone, PartialEq, Eq)]
pub struct KeyPair {
    public: PublicKey,
    secret: SecretKey,
}

impl KeyPair {
    /// Generate a new random `KeyPair`
    pub fn random() -> Self {
        let (public, secret) = gen_keypair();

        Self {
            public: PublicKey(public),
            secret: SecretKey(secret),
        }
    }
}

#[derive(Snafu, Debug)]
pub enum SealError {
    #[snafu(display("failed to encrypt data: {}", source))]
    SealEncryptError { source: EncryptError },

    #[snafu(display("failed to decrypt data: {}", source))]
    SealDecryptError { source: DecryptError },

    #[snafu(display("missing cryptographic header in box"))]
    MissingHeader { backtrace: Backtrace },

    #[snafu(display("invalid mac in sealed box"))]
    InvalidMac { backtrace: Backtrace },

    #[snafu(display("serializer error: {}", source))]
    SerializeError { source: Box<BincodeErrorKind> },
}

/// An asymmetric encryption/decryption structure. <br />
/// Message are in the following format:
/// The first 16 bytes are the tag of the message.
/// The following 24 are the single use Nonce that was used to seal the message
/// the rest is the actual ciphertext of the message.
pub struct Seal {
    keypair: KeyPair,
    buffer: Vec<u8>,
}

impl Seal {
    /// Create a new `Seal` using the specified `KeyPair`
    pub fn new(keypair: KeyPair) -> Self {
        Self {
            keypair,
            buffer: Vec::new(),
        }
    }

    /// Create a new `Seal` with a randomly generated `KeyPair`
    pub fn random() -> Self {
        Self::new(KeyPair::random())
    }

    /// Get a reference to the `PublicKey` in use within this `Seal`
    pub fn public(&self) -> &PublicKey {
        &self.keypair.public
    }

    /// Get a reference to the `SecretKey` in use within this `Seal`
    pub fn secret(&self) -> &SecretKey {
        &self.keypair.secret
    }

    /// Encrypts a serializable message using the public key
    /// given when creating this `Seal`.
    pub fn encrypt<T: Serialize>(
        &mut self,
        recipient_key: &PublicKey,
        message: &T,
    ) -> Result<Vec<u8>, SealError> {
        let nonce = gen_nonce();
        let mut output = Vec::new();
        let size = serialized_size(message).context(SerializeError)? as usize;

        output.resize_with(size + NONCE_LENGTH + TAG_LENGTH, || 0);

        serialize_into(&mut output[HEADER_LENGTH..], message)
            .context(SerializeError)?;

        let tag = seal_detached(
            &mut output[HEADER_LENGTH..],
            &nonce,
            &recipient_key.0,
            &self.keypair.secret.0,
        );

        output[..TAG_LENGTH].copy_from_slice(tag.0.as_ref());
        output[TAG_LENGTH..HEADER_LENGTH].copy_from_slice(nonce.0.as_ref());

        debug_assert_eq!(
            output.len(),
            output.capacity(),
            "unexpected ciphertext length"
        );

        Ok(output)
    }

    /// Decrypts a deserializable message from a slice of bytes without
    /// performing any copies. This means that the result value can't be used
    /// after this `Seal` instance is dropped. Moreover it is not possible to
    /// reuse this `Seal` instance before the resulting message has been dropped
    pub fn decrypt_ref<T>(
        &mut self,
        sender_key: &PublicKey,
        ciphertext: &[u8],
    ) -> Result<T, SealError>
    where
        for<'de> T: Deserialize<'de>,
    {
        ensure!(
            ciphertext.len() > TAG_LENGTH + NONCE_LENGTH,
            MissingHeader {}
        );

        let tag = SodiumTag::from_slice(&ciphertext[0..TAG_LENGTH])
            .ok_or_else(|| InvalidMac {}.build())?;
        let nonce =
            SodiumNonce::from_slice(&ciphertext[TAG_LENGTH..HEADER_LENGTH])
                .ok_or_else(|| InvalidMac.build())?;

        self.buffer.clear();
        self.buffer.extend_from_slice(&ciphertext[HEADER_LENGTH..]);

        open_detached(
            &mut self.buffer,
            &tag,
            &nonce,
            &sender_key.0,
            &self.keypair.secret.0,
        )
        .map_err(|_| InvalidMac {}.build())?;

        Ok(deserialize(&self.buffer).context(SerializeError)?)
    }

    /// Decrypts a serializable message from a slice of bytes. This method is
    /// similar to Seal::decrypt_ref except that it copies data as needed to
    /// produce a T that is owned by the caller.
    pub fn decrypt<T>(
        &mut self,
        sender_key: &PublicKey,
        ciphertext: &[u8],
    ) -> Result<T, SealError>
    where
        for<'de> T: Deserialize<'de> + ToOwned<Owned = T>,
    {
        self.decrypt_ref(sender_key, ciphertext)
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Debug;

    use super::*;

    macro_rules! encrypt_decrypt_gen {
        ($func:ident, $value:expr) => {
            let mut sealer = Seal::random();
            let mut opener = Seal::random();

            let bytes = sealer
                .encrypt(opener.public(), &($value))
                .expect("encryption failed");

            let x: T = opener
                .$func(sealer.public(), &bytes)
                .expect("failed to decipher");

            assert_eq!(x, ($value), "decryption did not yield same data");
        };
    }

    fn encrypt_decrypt_ref_compare<T>(value: T)
    where
        for<'de> T: Serialize + Deserialize<'de> + Eq + Debug,
    {
        encrypt_decrypt_gen!(decrypt_ref, value);
    }

    fn encrypt_decrypt_copy_compare<T>(value: T)
    where
        for<'de> T: Serialize + Deserialize<'de> + Eq + Debug + Clone,
    {
        encrypt_decrypt_gen!(decrypt, value);
    }

    #[test]
    fn size_assert() {
        assert_eq!(TAG_LENGTH, 16, "sodium tag length has changed");
        assert_eq!(NONCE_LENGTH, 24, "sodium nonce length has changed");
        assert_eq!(PUBLIC_LENGTH, 32, "sodium key length has changed");
        assert_eq!(SECRET_LENGTH, 32, "sodium key length has changed");
    }

    #[test]
    fn correct_encrypt_decrypt_ref_u64() {
        encrypt_decrypt_ref_compare(0u64);
    }

    #[test]
    fn correct_encrypt_decrypt_ref_u8_slice() {
        encrypt_decrypt_ref_compare([0x00, 0x01, 0x02, 0x03, 0x04]);
    }

    #[test]
    fn correct_encrypt_decrypt_ref_str() {
        encrypt_decrypt_ref_compare("Hello World!".to_string());
    }

    #[test]
    fn encrypt_decrypt_ref_tuple() {
        let s: (u8, u16, u32) = rand::random();

        encrypt_decrypt_ref_compare(s);
    }

    #[test]
    fn encrypt_decrypt_copy_u64() {
        encrypt_decrypt_copy_compare(0u64);
    }

    #[test]
    fn encrypt_decrypt_copy_tuple() {
        let s: (u8, u16, u32) = rand::random();

        encrypt_decrypt_copy_compare(s);
    }

    #[test]
    fn encrypt_decrypt_copy_slice_u8() {
        let s: [u8; 32] = rand::random();

        encrypt_decrypt_copy_compare(s);
    }

    #[test]
    fn encrypt_decrypt_copy_str() {
        encrypt_decrypt_copy_compare("Hello World!".to_string());
    }

    #[test]
    fn wrong_key_decrypt() {
        let mut seal = Seal::random();
        let keypair1 = KeyPair::random();
        let keypair2 = KeyPair::random();

        let encrypted = seal
            .encrypt(&keypair1.public, &0u64)
            .expect("failed to encrypt data");

        seal.decrypt::<u64>(&keypair2.public, &encrypted)
            .expect_err("verified data with wrong public key");
    }

    #[test]
    fn missing_header_message() {
        let keypair = KeyPair::random();
        let mut seal = Seal::new(keypair.clone());
        let length = rand::random::<usize>() % TAG_LENGTH + NONCE_LENGTH;

        let data: Vec<u8> = (0..length).map(|_| rand::random()).collect();

        seal.decrypt_ref::<u8>(&keypair.public, &data)
            .expect_err("decrypted message without complete header");
    }

    #[test]
    fn bad_nonce() {
        let mut seal = Seal::random();
        let public = seal.public().clone();

        let mut encrypted =
            seal.encrypt(&public, &0u64).expect("failed to encrypt");

        encrypted[TAG_LENGTH + 2] = encrypted[TAG_LENGTH + 2].wrapping_add(2);

        seal.decrypt::<u64>(&public, &encrypted)
            .expect_err("decrypted corrupted message");
    }
}
