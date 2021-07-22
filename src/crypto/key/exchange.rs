use std::fmt;

use serde::{Deserialize, Serialize};
use snafu::{Backtrace, Snafu};
use sodiumoxide::crypto::kx::{
    client_session_keys, gen_keypair, server_session_keys,
    PublicKey as SodiumPubKey, SecretKey as SodiumSecKey,
};

use super::{
    super::stream::{Pull, Push},
    Key,
};

/// Error encountered while computing shared secrets using [`Exchanger`]
///
/// [`Exchanger`]: self::Exchanger
#[derive(Debug, Snafu)]
pub enum ExchangeError {
    /// Cryptographic operation failure
    #[snafu(display("sodium failure"))]
    Sodium {
        /// Error backtrace
        backtrace: Backtrace,
    },
}

#[derive(
    Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
/// A `PublicKey` used to compute a shared secret with a remote party
pub struct PublicKey(SodiumPubKey);

impl From<SodiumPubKey> for PublicKey {
    fn from(key: SodiumPubKey) -> Self {
        Self(key)
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

impl fmt::Display for PrivateKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for b in self.0.as_ref() {
            write!(f, "{:02x}", b)?;
        }
        Ok(())
    }
}

impl fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "PublicKey {{ ")?;
        <Self as fmt::Display>::fmt(self, f)?;
        write!(f, " }}")
    }
}

impl AsRef<[u8]> for PublicKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

#[derive(Clone, Serialize, Deserialize)]
/// A `PrivateKey` used to compute a shared secret with a remote party
pub struct PrivateKey(SodiumSecKey);

impl AsRef<[u8]> for PrivateKey {
    fn as_ref(&self) -> &[u8] {
        self.0.as_ref()
    }
}

impl From<SodiumSecKey> for PrivateKey {
    fn from(key: SodiumSecKey) -> Self {
        Self(key)
    }
}

#[derive(Clone, Deserialize, Serialize)]
/// A `KeyPair` that can be used to exchange a secret symmetric key for use in an encrypted network stream
pub struct KeyPair {
    public: PublicKey,
    secret: PrivateKey,
}

impl KeyPair {
    /// Creates a new `KeyPair` with a public key linked to the secret key
    pub fn new(secret: PrivateKey, public: PublicKey) -> Self {
        // TODO check that keys are linked somehow
        Self { public, secret }
    }

    /// Generate a new random `KeyPair`
    pub fn random() -> Self {
        let (public, secret) = gen_keypair();

        Self {
            public: PublicKey(public),
            secret: PrivateKey(secret),
        }
    }

    /// Get the `PublicKey` from this `KeyPair`
    pub fn public(&self) -> &PublicKey {
        &self.public
    }

    /// Get the `PrivateKey` from this `KeyPair`
    pub fn secret(&self) -> &PrivateKey {
        &self.secret
    }
}

/// A pair of exchanged ephemeral keys that can be used to
/// securely exchange data with a peer.
#[derive(Debug)]
pub struct Session {
    transmit: Key,
    receive: Key,
}

impl From<Session> for (Push, Pull) {
    fn from(session: Session) -> Self {
        (Push::new(session.transmit), Pull::new(session.receive))
    }
}

/// A structure used to compute a shared secret with another
/// party using a `KeyPair` and the other party's `PublicKey`
#[derive(Clone)]
pub struct Exchanger {
    keypair: KeyPair,
}

impl Exchanger {
    /// Create a new `KeyExchanger` using a provided `KeyPair`
    pub fn new(keypair: KeyPair) -> Self {
        Self { keypair }
    }

    /// Create a new `KeyExchanger` using a random `KeyPair`
    pub fn random() -> Self {
        Self {
            keypair: KeyPair::random(),
        }
    }

    /// Get a reference to the `KeyPair` used by this `KeyExchanger`
    pub fn keypair(&self) -> &KeyPair {
        &self.keypair
    }

    /// Exchange keys with a remote peer.
    /// The resulting `SessionKey` can be used to securely encrypt and decrypt
    /// data to and from the remote peer.
    pub fn exchange(
        &self,
        pubkey: &PublicKey,
    ) -> Result<Session, ExchangeError> {
        if *pubkey < self.keypair.public {
            server_session_keys(
                &self.keypair.public.0,
                &self.keypair.secret.0,
                &pubkey.0,
            )
        } else {
            client_session_keys(
                &self.keypair.public.0,
                &self.keypair.secret.0,
                &pubkey.0,
            )
        }
        .map(|(rx, tx)| Session {
            receive: rx.into(),
            transmit: tx.into(),
        })
        .map_err(|_| Sodium.build())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a KeyExchange from the given `KeyPair` and computes
    /// shared secret using the given `PublicKey`
    macro_rules! exchange_key {
        ($keypair:expr, $pubkey:expr) => {{
            let kp = $keypair;
            let pk = $pubkey;

            Exchanger::new(kp)
                .exchange(&pk)
                .expect("failed to compute secret")
        }};
    }

    #[test]
    fn valid_exchange() {
        let srv_keypair = KeyPair::random();
        let cli_keypair = KeyPair::random();

        let srv_session =
            exchange_key!(srv_keypair.clone(), cli_keypair.public);
        let cli_session = exchange_key!(cli_keypair, &srv_keypair.public);

        assert_eq!(
            srv_session.receive, cli_session.transmit,
            "the computed secret did not match"
        );
        assert_eq!(
            srv_session.transmit, cli_session.receive,
            "the computed secret did not match"
        );
    }

    #[test]
    fn invalid_public_key() {
        let (srv, cli) = (KeyPair::random(), KeyPair::random());
        let wrong_keypair = KeyPair::random();

        let srv_session = exchange_key!(srv, cli.public);
        let cli_session = exchange_key!(cli, *wrong_keypair.public());

        assert_ne!(
            cli_session.receive, srv_session.transmit,
            "computed same secret with different keys"
        );

        assert_ne!(
            cli_session.transmit, srv_session.receive,
            "computed same secret with different keys"
        );
    }
}
