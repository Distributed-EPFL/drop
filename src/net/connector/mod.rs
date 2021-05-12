/// Connector that uses a central directory server to find peers
mod directory;
pub use directory::Directory;

/// Connector that can use anything that resolves to a `SocketAddr`
mod resolve;
pub use resolve::Resolve;

/// Tcp related connectors
mod tcp;
pub use tcp::Direct as Tcp;

/// uTP connector
mod utp;
pub use self::utp::Direct as Utp;

use std::fmt;
use std::io::{Error, ErrorKind};

use super::{Connection, SecureError, Socket};
use crate::crypto::key::exchange::{Exchanger, PublicKey};

use async_trait::async_trait;

use futures::future;

use snafu::{ResultExt, Snafu};

use tracing::{debug_span, info};
use tracing_futures::Instrument;

#[derive(Debug, Snafu)]
/// Error encountered by [`Connector`] when attempting to establish a [`Connection`]
///
/// [`Connector`]: self::Connector
/// [`Connection`]: super::Connection
pub enum ConnectError {
    #[snafu(display("i/o error: {}", source))]
    #[snafu(visibility(pub))]
    /// OS error when connecting
    Io {
        /// Underlying error cause
        source: Error,
    },
    #[snafu(display("could not secure connection: {}", source))]
    #[snafu(visibility(pub))]
    /// Error encountered when attempting to secure an outgoing `Connection`
    Secure {
        /// Underlying error cause
        source: SecureError,
    },
    #[snafu(display("underlying connector error: {}", reason))]
    #[snafu(visibility(pub))]
    /// Any other kind of error
    Other {
        /// Details about what failed
        reason: String,
    },
}

impl From<ErrorKind> for ConnectError {
    fn from(kind: ErrorKind) -> Self {
        use snafu::IntoError;

        Io {}.into_error(kind.into())
    }
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

/// An extension trait for [`Connector`]s
///
/// [`Connector`]: self::Connector
/// [`BackoffConnector`]: self::BackoffConnector
/// [`Resolve`]: super::Resolve
pub trait ConnectorExt: Connector + Sized {
    /// Wrap the [`Connector`] into a [`BackoffConnector`]
    fn retry(self) -> BackoffConnector<Self> {
        BackoffConnector::new(self)
    }

    /// Wrap the [`Connector`] into a [`Resolve`]
    fn resolve(self) -> Resolve<Self, Self::Candidate> {
        Resolve::new(self)
    }
}

impl<C> ConnectorExt for C where C: Connector {}

/// Retry a [`Connector`] using exponential backoff
///
/// [`Connector`]: self::Connector
pub struct BackoffConnector<C>
where
    C: Connector,
{
    connector: C,
}

impl<C> BackoffConnector<C>
where
    C: Connector,
{
    fn new(connector: C) -> Self {
        Self { connector }
    }
}

#[async_trait]
impl<C> Connector for BackoffConnector<C>
where
    C: Connector,
{
    type Candidate = C::Candidate;

    fn exchanger(&self) -> &Exchanger {
        self.connector.exchanger()
    }

    async fn establish(
        &self,
        pkey: &PublicKey,
        candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        backoff::future::retry(
            backoff::ExponentialBackoff::default(),
            || async {
                let stream = self.connector.establish(pkey, candidate).await?;
                Ok(stream)
            },
        )
        .await
    }
}
