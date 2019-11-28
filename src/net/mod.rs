/// Utilities to connect to other peers in a secure fashion
pub mod connector;

/// Utilities to open a listener for incoming connections
pub mod listener;

use crate as drop;
use crate::error::Error;

use async_trait::async_trait;

use macros::error;

use tokio::io::Error as TokioErr;

error! {
    type: TokioError,
    description: "tokio encountered an error",
    causes: (TokioErr),
}

/// Trait for structs that are able to asynchronously send and receive data from
/// the network
#[async_trait]
pub trait Connection {
    /// The type of `Error` returned when this `Connection` fails
    type Error;

    /// Asynchronously receive a `Deserialize` value from this `Connection`
    async fn receive(&mut self, buf: &mut [u8]) -> Result<usize, Self::Error>;

    /// Asynchronously send a `Serialize` message on this `Connection`
    async fn send(&mut self, buf: &[u8]) -> Result<usize, Self::Error>;
}
