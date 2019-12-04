/// Tcp related connectors
pub mod tcp;

use std::io::Error as IoError;

use super::{Connection, SecureError, Socket};
use crate as drop;
use crate::crypto::key::exchange::{Exchanger, PublicKey};
use crate::crypto::ExchangeError;

use async_trait::async_trait;

use macros::error;

use tokio::io::Error as TokioError;
use tokio::net::ToSocketAddrs;

error! {
    type: ConnectError,
    description: "error opening connection",
    causes: (TokioError, ExchangeError, SecureError, IoError)
}

/// The `Connector` trait is used to connect to peers using some sort of
/// Internet address (e.g. Ipv4 or Ipv6).
#[async_trait]
pub trait Connector {
    /// The target address type used by this connector
    type Candidate: ToSocketAddrs;

    /// Connect asynchronously to a given destination with its `PublicKey` and
    /// the local node's `KeyExchanger` that has been passed when constructing
    /// the `Connector`
    async fn connect(
        &self,
        pkey: &PublicKey,
        candidate: Self::Candidate,
    ) -> Result<Connection, ConnectError> {
        let socket = Self::establish(&pkey, candidate).await?;
        let mut connection = Connection::new(socket);

        connection.secure_server(self.exchanger(), pkey).await?;

        Ok(connection)
    }

    /// Returns a reference to the `Exchanger` that should be used to
    /// secure `Connection`s
    fn exchanger(&self) -> &Exchanger;

    /// Establish a `Socket` to the given `Candidate` destination.
    /// This function should only open the connection and not send any data
    /// after the connection has been established
    async fn establish(
        pkey: &PublicKey,
        candidate: Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError>;

    /// Connect to any of the provided `Candidate` that advertise the
    /// given `PublicKey`
    async fn connect_any(
        pkey: &PublicKey,
        candidates: &[Self::Candidate],
    ) -> Result<Connection, ConnectError> {
        unimplemented!()
    }
}
