use std::fmt;

use super::super::stream::{Pull, Push};
use super::Key;

use serde::{Deserialize, Serialize};

use snafu::{Backtrace, Snafu};

use sodiumoxide::crypto::kx::{
    client_session_keys, gen_keypair, server_session_keys,
    PublicKey as SodiumPubKey, SecretKey as SodiumSecKey,
};

#[derive(Debug, Snafu)]
pub enum ExchangeError {
    #[snafu(display("sodium failure"))]
    Sodium { backtrace: Backtrace },
}

#[derive(
    Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize,
)]
/// A `PublicKey` used to compute a shared secret with a remote party
pub struct PublicKey(SodiumPubKey);

impl fmt::Display for PublicKey {
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

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
/// A `SecretKey` used to compute a shared secret with a remote party
pub struct SecretKey(SodiumSecKey);

#[derive(Clone, Eq, PartialEq)]
/// A `KeyPair` that can be used to exchange a secret symmetric key
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

    /// Get the `PublicKey` from this `KeyPair`
    pub fn public(&self) -> &PublicKey {
        &self.public
    }

    /// Get the `SecretKey` from this `KeyPair`
    pub fn secret(&self) -> &SecretKey {
        &self.secret
    }
}

/// A pair of exchanged ephemeral keys that can be used to
/// securely exchange data with a peer.
#[derive(Debug, Eq, PartialEq)]
pub struct Session {
    transmit: Key,
    receive: Key,
}

impl Into<(Push, Pull)> for Session {
    fn into(self) -> (Push, Pull) {
        (Push::new(self.transmit), Pull::new(self.receive))
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
    /// shared secret using the given `SecretKey`
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
