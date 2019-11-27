pub mod tcp;

use std::future::Future;

use crate::crypto::Key as PublicKey;

use tokio::net::ToSocketAddrs;

/// The `Connector` trait is used to connect to peers using some sort of
/// Internet address (e.g. Ipv4 or Ipv6).
pub trait Connector {
    /// The address type used by this connector
    type Addr: ToSocketAddrs;

    /// The concrete type of `Connection` that this `Connector` will produce
    type Connection;

    /// Connect asynchronously to a given destination using the specifed
    /// `PublicKey`
    fn connect(
        addr: &[Self::Addr],
        pkey: &PublicKey,
    ) -> Box<dyn Future<Output = Self::Connection>>;
}
