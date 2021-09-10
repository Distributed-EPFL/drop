//! Aggregated cryptographic signatures using the BLS algorithm
//!
//! A convenient [`PublicKeySet`] is provided for handling the set of public keys with which to verify aggregate
//! [`Signature`]s
//!
//! ```
//! # use drop::crypto::bls::{PublicKey, AggregateSignature, AggregatePublicKey};
//! # fn doc(public_keys_iter: impl Iterator<Item = PublicKey>, signature: AggregateSignature, message: u8) {
//! let key_set = public_keys_iter.collect::<AggregatePublicKey>();
//! signature.verify(&message, &key_set).unwrap();
//! # }
//! ```
//!
//! Verification is done through the [`Signature`] struct directly
//!
//! ```
//! # use drop::crypto::bls::{Signature, AggregatePublicKey, AggregateSignature};
//! # fn doc(signature: AggregateSignature, pkeys: AggregatePublicKey, message: usize) {
//! signature.verify(&message, &pkeys).unwrap();
//! # }
//! ```
//!
//! [`Iterator`]: std::iter::Iterator
//! [`AggregatePublicKey`]: self::AggregatePublicKey
//! [`Signature`]: self::Signature

use std::fmt;
use std::iter::FromIterator;

use super::BincodeError;

use bincode::serialize_into;

use blst::min_sig::{
    AggregateSignature as BlsAggrSig, PublicKey as BlsPublicKey,
    SecretKey as BlsPrivateKey, Signature as BlsSignature,
};
use blst::BLST_ERROR;

use serde::{de, Deserialize, Deserializer, Serialize};

use snafu::{OptionExt, ResultExt, Snafu};

use rand::{rngs::OsRng, RngCore};

const BLST_DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";

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
        source: BlstError,
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

trait ToResult {
    type Err;

    fn into_result<S>(self, succ: S) -> Result<S, Self::Err>;
}

#[derive(Debug)]
/// An error from the blst crate
pub struct BlstError(BLST_ERROR);

impl ToResult for BLST_ERROR {
    type Err = BlstError;

    fn into_result<S>(self, succ: S) -> Result<S, BlstError> {
        if self == BLST_ERROR::BLST_SUCCESS {
            Ok(succ)
        } else {
            Err(self.into())
        }
    }
}

impl std::error::Error for BlstError {}

impl From<BLST_ERROR> for BlstError {
    fn from(v: BLST_ERROR) -> Self {
        Self(v)
    }
}

impl fmt::Display for BlstError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self.0 {
            BLST_ERROR::BLST_POINT_NOT_IN_GROUP => "point not in group",
            BLST_ERROR::BLST_AGGR_TYPE_MISMATCH => "signature type mismatched",
            BLST_ERROR::BLST_PK_IS_INFINITY => "public key is infinity",
            BLST_ERROR::BLST_SUCCESS => "no error",
            BLST_ERROR::BLST_BAD_ENCODING => "bad encoding",
            BLST_ERROR::BLST_POINT_NOT_ON_CURVE => "point not on curve",
            BLST_ERROR::BLST_VERIFY_FAIL => "bad signature",
        };

        write!(f, "{}", s)
    }
}

/// A `PrivateKey` for aggregated signatures
#[derive(Clone, Debug)]
pub struct PrivateKey(BlsPrivateKey);

impl PrivateKey {
    /// Create a new `PrivateKey` using a specified seed
    pub fn new<B>(bytes: B) -> Result<Self, BlsError>
    where
        B: AsRef<[u8]>,
    {
        BlsPrivateKey::from_bytes(bytes.as_ref())
            .map_err(Into::into)
            .context(Bls)
            .map(Into::into)
    }

    /// Generate a random `PrivateKey`
    pub fn random() -> Result<Self, BlsError> {
        let mut seed = [0; 32];
        OsRng.fill_bytes(&mut seed);

        BlsPrivateKey::key_gen(&seed, &[])
            .map_err(Into::into)
            .context(Bls)
            .map(Into::into)
    }

    /// Get the content of this `PrivateKey` as a `Vec` of bytes
    pub fn to_vec(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    /// Sign a message using this `PrivateKey`
    ///
    /// # Example
    ///
    /// ```
    /// # use drop::crypto::bls::PrivateKey;
    ///
    /// let key = PrivateKey::random().unwrap();
    /// let signature = key.sign(&0usize).expect("sign failed");
    ///
    /// signature.aggregate().verify(&[0usize], &key.public().into()).unwrap();
    /// ```
    pub fn sign<T>(&self, message: &T) -> Result<Signature, BlsError>
    where
        T: Serialize,
    {
        let mut buffer = Vec::new();

        serialize_into(&mut buffer, message).context(Serializer)?;

        Ok(self.0.sign(buffer.as_slice(), BLST_DST, &[]).into())
    }

    /// Get the [`PublicKey`] associated with this `PrivateKey`
    ///
    /// [`PublicKey`]: self::PublicKey
    pub fn public(&self) -> PublicKey {
        self.0.sk_to_pk().into()
    }
}

impl PartialEq for PrivateKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bytes() == other.0.to_bytes()
    }
}

impl Eq for PrivateKey {}

impl From<BlsPrivateKey> for PrivateKey {
    fn from(key: BlsPrivateKey) -> Self {
        Self(key)
    }
}

impl Serialize for PrivateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0.to_bytes())
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
                f.write_str("byte representation of a bls public key")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                BlsPrivateKey::from_bytes(v)
                    .map_err(Into::into)
                    .context(Bls)
                    .map_err(E::custom)
            }
        }

        Ok(Self(deserializer.deserialize_bytes(ByteVisitor)?))
    }
}

/// A BLS `PublicKey`
#[derive(Clone, Debug)]
pub struct PublicKey(BlsPublicKey);

impl PublicKey {
    /// Aggregate this `PublicKey`
    pub fn aggregate(self) -> AggregatePublicKey {
        AggregatePublicKey(vec![self.0])
    }
}

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
                BlsPublicKey::from_bytes(v)
                    .map_err(Into::into)
                    .context(Bls)
                    .map_err(E::custom)
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
        serializer.serialize_bytes(&self.0.to_bytes())
    }
}

impl PartialEq for PublicKey {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bytes() == other.0.to_bytes()
    }
}

impl From<BlsPublicKey> for PublicKey {
    fn from(k: BlsPublicKey) -> Self {
        Self(k)
    }
}

/// An aggregation of many different [`PublicKey`]s
///
/// [`PublicKey`]: self::PublicKey
#[derive(Clone)]
pub struct AggregatePublicKey(Vec<BlsPublicKey>);

impl AggregatePublicKey {
    /// Add a new [`PublicKey`] to this aggregation
    pub fn add(&mut self, other: PublicKey) {
        self.0.push(other.0)
    }

    fn as_slice(&self) -> &[BlsPublicKey] {
        self.0.as_slice()
    }
}

impl From<PublicKey> for AggregatePublicKey {
    fn from(k: PublicKey) -> Self {
        Self(vec![k.0])
    }
}

impl FromIterator<PublicKey> for AggregatePublicKey {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = PublicKey>,
    {
        Self(iter.into_iter().map(|x| x.0).collect())
    }
}

/// A BLS `Signature`
#[derive(Clone)]
pub struct Signature(BlsSignature);

impl Signature {
    /// Aggregate two `Signature`s together
    pub fn aggregate(self) -> AggregateSignature {
        BlsAggrSig::from_signature(&self.0).into()
    }

    /// Aggregate this `Signature` with another one
    ///
    /// # Example
    /// ```
    /// # use drop::crypto::bls::{AggregateSignature, Signature};
    /// # fn doc(sig1: Signature, sig2: AggregateSignature) {
    /// let aggregated = sig1.aggregate_other(sig2).expect("failed to aggregate");
    /// # }
    /// ```
    pub fn aggregate_other(
        &self,
        mut other: AggregateSignature,
    ) -> Result<AggregateSignature, BlsError> {
        other
            .0
            .add_signature(&self.0, false)
            .map_err(Into::into)
            .context(Bls)?;

        Ok(other)
    }

    /// Aggregate an `Iterator` of `Signature` into an `AggregateSignature`
    pub fn aggregate_iter<I>(iter: I) -> Result<AggregateSignature, BlsError>
    where
        I: IntoIterator<Item = Self>,
    {
        AggregateSignature::aggregate_iter(iter)
    }
}

impl PartialEq for Signature {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bytes() == other.0.to_bytes()
    }
}

impl Eq for Signature {
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Visitor;

        struct ByteVisitor;

        impl<'de> Visitor<'de> for ByteVisitor {
            type Value = BlsSignature;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("byte representation of a bls signature")
            }

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                BlsSignature::from_bytes(v)
                    .map_err(Into::into)
                    .context(Bls)
                    .map_err(E::custom)
            }
        }

        Ok(Self(deserializer.deserialize_bytes(ByteVisitor)?))
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0.to_bytes())
    }
}

impl From<BlsSignature> for Signature {
    fn from(signature: BlsSignature) -> Self {
        Self(signature)
    }
}

/// An aggregation of many different signature into a single one
#[derive(Clone)]
pub struct AggregateSignature(BlsAggrSig);

impl AggregateSignature {
    /// Aggregate one more signature into this `AggregateSignature`
    pub fn aggregate(&mut self, new: &Signature) -> Result<(), BlsError> {
        self.0
            .add_signature(&new.0, true)
            .map_err(Into::into)
            .context(Bls)
    }

    /// Aggregate two aggregate signature
    pub fn aggregate_agg(&mut self, other: &Self) {
        self.0.add_aggregate(&other.0)
    }

    /// Aggregate and `Iterator` of `Signature` into a single `AggregatedSignature`
    pub fn aggregate_iter<I>(iter: I) -> Result<Self, BlsError>
    where
        I: IntoIterator<Item = Signature>,
    {
        let mut iter = iter.into_iter();
        let first = iter.next().context(EmptySignature)?;

        iter.try_fold(first.aggregate(), |mut acc, curr| {
            acc.aggregate(&curr)?;

            Ok(acc)
        })
    }

    /// Attempt to verify that this signature is valid for the selected messages and public keys
    ///
    /// # Note
    ///
    /// `PublicKey`s in the `AggregatePublicKey` are required to be in the same order as the messages
    /// they were used to sign or the verification will fail
    ///
    /// # Example
    /// ```
    /// # use drop::crypto::bls::{PrivateKey, Signature, AggregatePublicKey};
    /// let private = (0..10).map(|_| PrivateKey::random().unwrap()).collect::<Vec<_>>();
    /// let public = private.iter().map(PrivateKey::public).collect::<AggregatePublicKey>();
    /// let messages = (0..10).collect::<Vec<_>>();
    ///
    /// let signatures = messages.iter().zip(private.iter())
    ///     .map(|(m, k)| k.sign(m).expect("sign failed"))
    ///     .collect::<Vec<_>>();
    /// let aggregated = Signature::aggregate_iter(signatures).expect("aggregate failed");
    ///
    /// aggregated.verify_many(messages.as_slice(), &public).unwrap();
    /// ```
    pub fn verify_many<T>(
        &self,
        messages: &[T],
        keys: &AggregatePublicKey,
    ) -> Result<(), BlsError>
    where
        T: Serialize,
    {
        let buffers = messages
            .iter()
            .map(|x| {
                let mut buffer = Vec::new();
                serialize_into(&mut buffer, x).expect("serialize failed");

                buffer
            })
            .collect::<Vec<_>>();

        let buffers_ref =
            buffers.iter().map(|x| x.as_slice()).collect::<Vec<_>>();
        let keys_refs = keys.as_slice().iter().collect::<Vec<_>>();

        self.0
            .to_signature()
            .aggregate_verify(
                true,
                buffers_ref.as_slice(),
                BLST_DST,
                keys_refs.as_slice(),
                true,
            )
            .into_result(())
            .context(Bls)
    }

    /// Attempt to verify that this signature is valid for the selected message and aggregated public keys
    ///
    /// # Note
    ///
    /// `PublicKey`s in the `AggregatePublicKey` are required to be in the same order as the messages
    /// they were used to sign or the verification will fail
    ///
    /// # Example
    /// ```
    /// # use drop::crypto::bls::{PrivateKey, Signature, AggregatePublicKey};
    /// let private = (0..10).map(|_| PrivateKey::random().unwrap()).collect::<Vec<_>>();
    /// let public = private.iter().map(PrivateKey::public).collect::<AggregatePublicKey>();
    /// let message = 0usize;
    ///
    /// let signatures = private.iter()
    ///     .map(|k| k.sign(&message).expect("sign failed"))
    ///     .collect::<Vec<_>>();
    /// let aggregated = Signature::aggregate_iter(signatures).expect("aggregate failed");
    ///
    /// aggregated.verify(&message, &public).unwrap();
    /// ```
    pub fn verify<T>(
        &self,
        message: &T,
        keys: &AggregatePublicKey,
    ) -> Result<(), BlsError>
    where
        T: Serialize,
    {
        let mut buffer = Vec::new();
        serialize_into(&mut buffer, message).expect("serialize failed");

        let keys_refs = keys.as_slice().iter().collect::<Vec<_>>();

        self.0
            .to_signature()
            .fast_aggregate_verify(
                true,
                buffer.as_slice(),
                BLST_DST,
                keys_refs.as_slice(),
            )
            .into_result(())
            .context(Bls)
    }
}

impl From<BlsAggrSig> for AggregateSignature {
    fn from(s: BlsAggrSig) -> Self {
        Self(s)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use bincode::deserialize_from;

    fn generate_sequence(
        size: usize,
    ) -> impl Iterator<Item = (usize, PrivateKey)> {
        (0..size).map(|x| (x, PrivateKey::random().unwrap()))
    }

    fn sign(
        count: usize,
    ) -> impl Iterator<Item = (usize, Signature, PrivateKey)> {
        generate_sequence(count)
            .map(|(m, k)| (m, k.sign(&m).expect("sign failed"), k))
    }

    fn sign_same<T: Serialize>(
        message: T,
        size: usize,
    ) -> impl Iterator<Item = (PublicKey, Signature)> {
        (0..size).map(move |_| {
            let key = PrivateKey::random().unwrap();

            (key.public(), key.sign(&message).unwrap())
        })
    }

    #[test]
    fn sign_aggregate_and_verify() {
        let all = sign(10).collect::<Vec<_>>();
        let keys = all.iter().map(|(_, _, k)| k);
        let mut signatures = all.iter().map(|(_, s, _)| s);
        let messages = all.iter().map(|(m, _, _)| m).collect::<Vec<_>>();

        let public =
            keys.map(PrivateKey::public).collect::<AggregatePublicKey>();
        let initial = signatures.next().map(Clone::clone).unwrap().aggregate();

        let aggregate = signatures.fold(initial, |mut acc, curr| {
            acc.aggregate(&curr).unwrap();
            acc
        });

        aggregate.verify_many(&messages, &public).expect("verify failed");
    }

    #[test]
    fn sign_aggregate_and_verify_single() {
        const MSG: usize = 0;
        let keys = (0..10)
            .map(|_| PrivateKey::random().unwrap())
            .collect::<Vec<_>>();

        let sigs = keys.iter().map(|k| k.sign(&MSG).unwrap());

        let aggr = Signature::aggregate_iter(sigs).unwrap();
        let aggr_key = keys
            .into_iter()
            .map(|k| k.public())
            .collect::<AggregatePublicKey>();

        aggr.verify_many(&[MSG; 10], &aggr_key).expect("failed");
    }

    #[test]
    fn sign_single_and_aggregate() {
        const MSG: usize = 0;

        let (keys, sigs): (Vec<_>, Vec<_>) = sign_same(MSG, 10).unzip();

        let aggr_sig = Signature::aggregate_iter(sigs.into_iter()).unwrap();
        let aggr_key = keys.into_iter().collect::<AggregatePublicKey>();

        aggr_sig.verify(&MSG, &aggr_key).expect("verify failed");
    }

    #[test]
    fn aggregate_signature() {
        let mut iter = sign(10);
        let (_, s1, _) = iter.next().unwrap();
        let s2 = Signature::aggregate_iter(iter.map(|(_, s, _)| s)).unwrap();

        s1.aggregate_other(s2).unwrap();
    }

    #[test]
    fn aggregate_pkey() {
        let mut pkeys = generate_sequence(10).map(|(_, k)| k.public());
        let mut agg: AggregatePublicKey = pkeys.next().unwrap().into();

        for pkey in pkeys {
            agg.add(pkey);
        }
    }

    #[test]
    fn serialize_deserialize() {
        use std::io::Cursor;

        let key = PrivateKey::random().unwrap();
        let mut buffer = Vec::new();

        serialize_into(&mut buffer, &key).expect("serialize failed");

        let dkey =
            deserialize_from(Cursor::new(buffer)).expect("deserialize failed");

        assert_eq!(key, dkey, "wrong key");

        let pkey = key.public();
        let mut buffer = Vec::new();

        serialize_into(&mut buffer, &pkey).expect("serialize failed");

        let dpkey =
            deserialize_from(Cursor::new(buffer)).expect("deserialize failed");

        assert_eq!(pkey, dpkey, "wrong pubkey");
    }

    #[test]
    fn bad_deserialize() {
        use std::io::Cursor;
        let bad = [0u8; 32];

        let key: Result<PrivateKey, _> = deserialize_from(Cursor::new(bad));

        key.expect_err("deserialized garbage");
    }
}
