/// Tcp related connectors
pub mod tcp;

/// uTP connectors
pub mod utp;

mod directory;
pub use directory::*;

use std::fmt;
use std::io::Error as IoError;

use super::{Connection, SecureError, Socket};
use crate as drop;
use crate::crypto::key::exchange::{Exchanger, PublicKey};
use crate::crypto::ExchangeError;
use crate::error::Error;

use async_trait::async_trait;

use macros::error;

use tracing::{debug_span, info};
use tracing_futures::Instrument;

error! {
    type: ConnectError,
    description: "error opening connection",
    causes: (IoError, ExchangeError, SecureError)
}

/// The `Connector` trait is used to connect to peers using some sort of
/// Internet address (e.g. Ipv4 or Ipv6).
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
    /// * `candidate` - A remote address for the remote peer, the concrete type
    /// depends on the actual `Connector` used
    async fn connect(
        &mut self,
        pkey: &PublicKey,
        candidate: &Self::Candidate,
    ) -> Result<Connection, ConnectError> {
        let socket = self
            .establish(candidate)
            .instrument(debug_span!("establish"))
            .await?;
        let mut connection = Connection::new(socket);

        info!("connected to {}, exchanging keys", candidate);

        connection
            .secure_server(self.exchanger(), pkey)
            .instrument(debug_span!("key_exchange"))
            .await?;

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
        candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError>;

    /// Connect to any of the provided `Candidate` that advertise the
    /// given `PublicKey`. Only returns a `Connection` to the fastest
    /// responding `Candidate`
    async fn connect_any(
        &self,
        _pkey: &PublicKey,
        _candidates: &[Self::Candidate],
    ) -> Result<Connection, ConnectError> {
        unimplemented!() // FIXME: need join! from tokio 0.1 to be ported
    }
}
