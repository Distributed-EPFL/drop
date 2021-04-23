mod directory;
pub use directory::*;

mod tcp;
pub use tcp::*;

mod utp;
pub use self::utp::*;

use std::fmt;
use std::io::Error;

use super::{Connection, SecureError, Socket};
use crate::crypto::key::exchange::{Exchanger, PublicKey};

use async_trait::async_trait;

use snafu::{ResultExt, Snafu};

use tracing::{debug_span, info};
use tracing_futures::Instrument;

#[derive(Debug, Snafu)]
pub enum ConnectError {
    #[snafu(display("i/o error: {}", source))]
    #[snafu(visibility(pub))]
    Io { source: Error },
    #[snafu(display("could not secure connection: {}", source))]
    #[snafu(visibility(pub))]
    Secure { source: SecureError },
}

/// The `Connector` trait is used to connect to peers using some `Candidate`.
#[async_trait]
pub trait Connector: Send + Sync {
    /// The target address type used by this connector
    type Candidate: Send + Sync + fmt::Display;

    /// Connect asynchronously to a given destination with its `PublicKey` and
    /// the local node's `KeyExchanger` that has been passed when constructing
    /// the `Connector`
    ///
    /// # Arguments
    /// * `pkey` - Public key of the remote peer we are connectiong to
    /// * `candidate` - Information needed to connect to the remote peer,
    /// the concrete type depends on the actual `Connector` used
    async fn connect(
        &mut self,
        pkey: &PublicKey,
        candidate: &Self::Candidate,
    ) -> Result<Connection, ConnectError> {
        let socket = self
            .establish(pkey, candidate)
            .instrument(debug_span!("establish"))
            .await?;
        let mut connection = Connection::new(socket);

        info!("connected to {}, exchanging keys", candidate);

        connection
            .secure_server(self.exchanger(), pkey)
            .instrument(debug_span!("key_exchange"))
            .await
            .context(Secure)?;

        info!("secure connection established with {}", candidate);

        Ok(connection)
    }

    /// Returns a reference to the `Exchanger` that should be used to
    /// secure `Connection`s
    fn exchanger(&self) -> &Exchanger;

    /// Establish a `Socket` to the given `Candidate` destination.
    /// This function should only open the connection and not send any data
    /// after the connection has been established in order not to make the
    /// remote end close the connection.
    async fn establish(
        &mut self,
        pkey: &PublicKey,
        candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError>;

    /// Connect to any of the provided `Candidate` that advertise the
    /// given `PublicKey`. Only returns a `Connection` to the fastest
    /// responding `Candidate`
    async fn connect_any(
        &mut self,
        _pkey: &PublicKey,
        _candidates: &[Self::Candidate],
    ) -> Result<Connection, ConnectError> {
        unimplemented!() // FIXME: need join! from tokio 0.1 to be ported
    }
}
