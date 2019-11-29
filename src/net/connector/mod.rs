/// Tcp related connectors
pub mod tcp;

use crate::crypto::key::exchange::{Exchanger, PublicKey};

use async_trait::async_trait;

use bincode::ErrorKind as BincodeErrorKind;

use tokio::net::ToSocketAddrs;

pub type SerializerError = Box<BincodeErrorKind>;

/// The `Connector` trait is used to connect to peers using some sort of
/// Internet address (e.g. Ipv4 or Ipv6).
#[async_trait]
pub trait Connector {
    /// The target address type used by this connector
    type Addr: ToSocketAddrs;

    /// The concrete type of `Connection` that this `Connector` will produce
    type Connection;

    /// The type of error that this `Connector` will return
    type Error;

    /// Connect asynchronously to a given destination with its `PublicKey` and
    /// the local node's `KeyExchanger`
    async fn connect(
        addr: Self::Addr,
        exchanger: Exchanger,
        pkey: &PublicKey,
    ) -> Result<Self::Connection, Self::Error>;
}
