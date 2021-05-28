//! This module provides utilities for aggregated cryptographic signatures using the BLS algorithm
//!

use std::fmt;
use std::iter::FromIterator;
use std::str::FromStr;

use super::BincodeError;

use bincode::serialize_into;

use bls_signatures::{
    Error, PrivateKey as BlsPrivateKey, PublicKey as BlsPublicKey,
    Serialize as _, Signature as BlsSignature,
};

use serde::{de, Deserialize, Deserializer, Serialize};

use snafu::{OptionExt, ResultExt, Snafu};

use rand::rngs::OsRng;


#[derive(Debug, Snafu)]
/// Type of error encountered when dealing with bls [`Signature`] and [`PrivateKey`]
///
/// [`Signature`]: self::Signature
/// [`PrivateKey`]: self::PrivateKey
pub enum BlsError {
    #[snafu(display("bls library error: {}", source))]
    /// Error encountered by the bls signature library
    Bls {
        /// BLS library error
        source: Error,
    },

    #[snafu(display("error serializing data: {}", source))]
    /// Serialization error
    Serializer {
        /// Underlying serializer error
        source: BincodeError,
    },

    #[snafu(display("empty signature list"))]
    /// List of provided [`Signature`] is empty
    EmptySignature,
}

/// A `PrivateKey` for aggregated signatures
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct PrivateKey(BlsPrivateKey);

impl PrivateKey {
    /// Create a new `PrivateKey` using a specified seed
    pub fn new<B>(bytes: B) -> Result<Self, BlsError>
    where
        B: AsRef<[u8]>,
    {
        BlsPrivateKey::from_bytes(bytes.as_ref())
            .context(Bls)
            .map(Into::into)
    }

    /// Generate a random `PrivateKey`
    pub fn random() -> Self {
        BlsPrivateKey::generate(&mut OsRng).into()
    }

    /// Get the content of this `PrivateKey` as a `Vec` of bytes
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.as_bytes()
    }

    /// Sign a message using this `PrivateKey`
    ///
    /// # Example
    ///
    /// ```
    /// # use drop::crypto::bls::PrivateKey;
    ///
    /// let key = PrivateKey::random();
    /// let signature = key.sign(&0usize).expect("sign failed");
    ///
    /// assert!(signature.verify(&[0usize], &key.public().into()).unwrap());
    /// ```
    pub fn sign<T>(&self, message: &T) -> Result<Signature, BlsError>
    where
        T: Serialize,
    {
        let mut buffer = Vec::new();

        serialize_into(&mut buffer, message).context(Serializer)?;

        Ok(self.0.sign(buffer).into())
    }

    /// Get the [`PublicKey`] associated with this `PrivateKey`
    ///
    /// [`PublicKey`]: self::PublicKey
    pub fn public(&self) -> PublicKey {
        self.0.public_key().into()
    }
}

impl From<BlsPrivateKey> for PrivateKey {
    fn from(key: BlsPrivateKey) -> Self {
        Self(key)
    }
}

/// A BLS `PublicKey`
pub struct PublicKey(BlsPublicKey);

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct ByteVisitor;

        impl<'de> Visitor<'de> for ByteVisitor {
            type Value = BlsPublicKey;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("byte representation of a bls public key")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                BlsPublicKey::from_bytes(v).map_err(E::custom)
            }
        }

        Ok(Self(deserializer.deserialize_bytes(ByteVisitor)?))
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.0.as_bytes().as_slice())
    }
}

impl From<BlsPublicKey> for PublicKey {
    fn from(k: BlsPublicKey) -> Self {
        Self(k)
    }
}

impl From<&BlsPublicKey> for PublicKey {
    fn from(k: &BlsPublicKey) -> Self {
        Self(*k)
    }
}

/// A set of [`PublicKey`]s for easier management when verifying aggregated [`Signature`]s
///
/// [`PublicKey`]: self::PublicKey
/// [`Signature`]: self::Signature
pub struct PublicKeySet(Vec<BlsPublicKey>);

impl PublicKeySet {
    /// Get an `Iterator` of all `PublicKey`s in this set
    pub fn iter(&self) -> impl Iterator<Item = PublicKey> + '_ {
        self.0.iter().map(Into::into)
    }

    /// Get the element at position idx in the set
    pub fn get(&self, idx: usize) -> Option<PublicKey> {
        self.0.get(idx).map(Into::into)
    }

    /// Add a `PublicKey` to this set
    pub fn insert(&mut self, key: PublicKey) {
        debug_assert!(!self.0.contains(&key.0), "key already in the set");

        self.0.push(key.0);
    }

    fn as_slice(&self) -> &[BlsPublicKey] {
        self.0.as_slice()
    }
}

impl From<PublicKey> for PublicKeySet {
    fn from(k: PublicKey) -> Self {
        Self(vec![k.0])
    }
}

impl FromIterator<PublicKey> for PublicKeySet {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = PublicKey>,
    {
        Self(iter.into_iter().map(|x| x.0).collect())
    }
}

impl FromStr for PrivateKey {
    type Err = BlsError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(BlsPrivateKey::from_string(s).context(Bls)?))
    }
}

impl<'de> Deserialize<'de> for PrivateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct ByteVisitor;

        impl<'de> Visitor<'de> for ByteVisitor {
            type Value = BlsPrivateKey;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("byte representation of a bls private key")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                BlsPrivateKey::from_bytes(v).map_err(E::custom)
            }
        }

        Ok(Self(deserializer.deserialize_bytes(ByteVisitor)?))
    }
}

impl Serialize for PrivateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.0.as_bytes().as_slice())
    }
}

/// An aggregated bls signature
pub struct Signature(BlsSignature);

impl Signature {
    /// Aggregate this `Signature` with another one
    ///
    /// # Example
    /// ```
    /// # use drop::crypto::bls::Signature;
    /// # fn doc(sig1: Signature, sig2: Signature) {
    /// let aggregated = sig1.aggregate(&sig2).expect("failed to aggregate");
    /// # }
    /// ```
    pub fn aggregate(&self, other: &Self) -> Result<Signature, BlsError> {
        bls_signatures::aggregate(&[self.0, other.0])
            .context(Bls)
            .map(Into::into)
    }

    /// Aggregate an [`Iterator`] of `Signature` into a single `Signature`
    ///
    /// [`Iterator`]: std::iter::Iterator
    pub fn aggregate_iter<I: IntoIterator<Item = Signature>>(
        iter: I,
    ) -> Result<Signature, BlsError>
    where
        I: IntoIterator<Item = Signature>,
    {
        let mut iter = iter.into_iter();

        let first = iter.next().context(EmptySignature)?;

        iter.try_fold(first, |acc, curr| acc.aggregate(&curr))
    }

    /// Attempt to verify that this signature is valid for the selected messages and public keys
    ///
    /// # Note
    ///
    /// This function will return `Ok(false)` if two of the messages are identical
    ///
    /// # Example
    /// ```
    /// # use drop::crypto::bls::{PrivateKey, Signature, PublicKeySet};
    /// let private = (0..10).map(|_| PrivateKey::random()).collect::<Vec<_>>();
    /// let public = private.iter().map(PrivateKey::public).collect::<PublicKeySet>();
    /// let messages = (0..10).collect::<Vec<_>>();
    ///
    /// let signatures = messages.iter().zip(private.iter())
    ///     .map(|(m, k)| k.sign(m).expect("sign failed"))
    ///     .collect::<Vec<_>>();
    /// let aggregated = Signature::aggregate_iter(signatures).expect("aggregate failed");
    ///
    /// assert!(aggregated.verify(messages.as_slice(), &public).unwrap());
    /// ```
    pub fn verify<T>(
        &self,
        messages: &[T],
        pkeys: &PublicKeySet,
    ) -> Result<bool, BlsError>
    where
        T: Serialize,
    {
        let mut buffer = Vec::new();

        let hashes = messages
            .iter()
            .map(|m| {
                buffer.clear();

                serialize_into(&mut buffer, m).context(Serializer)?;

                Ok(bls_signatures::hash(&buffer))
            })
            .try_collect::<Vec<_>>()?;

        Ok(bls_signatures::verify(&self.0, &hashes, pkeys.as_slice()))
    }
}

trait TryIterator<I, E>: Iterator<Item = Result<I, E>> + Sized {
    fn try_collect<C>(self) -> Result<C, E>
    where
        C: Extend<I> + Default,
    {
        let mut collection = C::default();

        for next in self {
            collection.extend(std::iter::once(next?));
        }

        Ok(collection)
    }
}

impl<I, O, E> TryIterator<O, E> for I where I: Iterator<Item = Result<O, E>> {}

impl From<BlsSignature> for Signature {
    fn from(signature: BlsSignature) -> Self {
        Self(signature)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use bincode::deserialize_from;

    fn generate_sequence(
        size: usize,
    ) -> impl Iterator<Item = (u32, PrivateKey)> {
        (0..size).map(|_| (0, PrivateKey::random()))
    }

    fn sign(
        count: usize,
    ) -> impl Iterator<Item = (u32, Signature, PrivateKey)> {
        generate_sequence(count)
            .map(|(m, k)| (m, k.sign(&m).expect("sign failed"), k))
    }

    #[test]
    fn sign_and_verify() {
        let (msg, signature, key) = sign(1).next().unwrap();

        assert!(signature
            .verify(&[msg], &key.public().into())
            .expect("verify failed"));
    }

    #[test]
    fn sign_aggregate_and_verify() {
        let (msg, keys): (Vec<_>, Vec<_>) = generate_sequence(10).unzip();
        let signature = Signature::aggregate_iter(
            msg.iter()
                .zip(keys.iter().copied())
                .map(|(m, k)| k.sign(m).expect("sign failed")),
        )
        .expect("aggregation failed");

        let public = keys
            .iter()
            .map(PrivateKey::public)
            .collect::<PublicKeySet>();

        signature.verify(&msg, &public).expect("verify failed");
    }

    #[test]
    fn serialize_deserialize() {
        use std::io::Cursor;

        let key = PrivateKey::random();
        let mut buffer = Vec::new();

        serialize_into(&mut buffer, &key).expect("serialize failed");

        let dkey =
            deserialize_from(Cursor::new(buffer)).expect("deserialize failed");

        assert_eq!(key, dkey, "wrong key");
    }

    #[test]
    fn bad_deserialize() {
        use std::io::Cursor;
        let bad = [0u8; 32];

        let key: Result<PrivateKey, _> = deserialize_from(Cursor::new(bad));

        key.expect_err("deserialized garbage");
    }
}
