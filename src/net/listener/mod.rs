/// Tcp related listeners utilities
pub mod tcp;

use super::Connection;

use async_trait::async_trait;

use tokio::net::ToSocketAddrs;

/// A trait used to accept incoming `Connection`s from other peers
#[async_trait]
pub trait Listener {
    /// The type of address that this `Listener` listens on
    type Addr: ToSocketAddrs;

    /// Type of incoming `Connection` returned by this `Listener`
    type Connection: Connection;

    /// Type of error returned by this `Listener`
    type Error;

    /// Asynchronously accept incoming `Connection`s
    async fn accept(&mut self) -> Result<Self::Connection, Self::Error>;
}
