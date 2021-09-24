use crate::{
    crypto::key::exchange::{Exchanger, PublicKey},
    net::{
        connect::errors::{ConnectError, Secure},
        socket::Socket,
        Connection,
    },
};

use async_trait::async_trait;

use futures::future;

use snafu::ResultExt;

use tracing::{debug_span, info};
use tracing_futures::Instrument;

/// The `Connector` trait is used to connect to peers using some `Candidate`.
#[async_trait]
pub trait Connector: Send + Sync {
    /// The target address type used by this connector
    type Candidate: Send + Sync + std::fmt::Display;

    /// Connect asynchronously to a given destination with its `PublicKey` and
    /// the local node's `KeyExchanger` that has been passed when constructing
    /// the `Connector`
    ///
    /// # Arguments
    /// * `pkey` - Public key of the remote peer we are connecting to
    /// * `candidate` - Information needed to connect to the remote peer,
    /// the concrete type depends on the actual `Connector` used
    async fn connect(
        &self,
        pkey: &PublicKey,
        candidate: &Self::Candidate,
    ) -> Result<Connection, ConnectError> {
        let socket = self
            .establish(pkey, candidate)
            .instrument(debug_span!("establish"))
            .await;

        let mut connection = Connection::new(socket?);

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
        &self,
        pkey: &PublicKey,
        candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError>;

    /// Connect to any of the provided `Candidate` that advertise the
    /// given `PublicKey`. Only returns a `Connection` to the fastest
    /// responding `Candidate`
    async fn connect_any(
        &self,
        pkey: &PublicKey,
        candidates: &[Self::Candidate],
    ) -> Result<Connection, ConnectError> {
        let futures = candidates.iter().map(|x| self.connect(pkey, x));

        future::select_ok(futures).await.map(|x| x.0)
    }

    /// Connect to many different peers using this `Connector`. All the
    /// `Connection`s will be established in parallel.
    async fn connect_many(
        &self,
        peers: &[(Self::Candidate, PublicKey)],
    ) -> Vec<Result<Connection, ConnectError>> {
        let futures = peers.iter().map(|(addr, pkey)| self.connect(pkey, addr));

        future::join_all(futures).await
    }
}
