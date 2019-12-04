/// Tcp related listeners utilities
pub mod tcp;

use super::{Connection, SecureError};
use crate as drop;
use crate::error::Error;

use async_trait::async_trait;

use macros::error;

use tokio::io::Error as TokioError;
use tokio::net::ToSocketAddrs;

error! {
    type: ListenerError,
    description: "error accepting incoming connection",
    causes: (TokioError, SecureError),
}

/// A trait used to accept incoming `Connection`s from other peers
#[async_trait]
pub trait Listener {
    /// The type of address that this `Listener` listens on
    type Candidate: ToSocketAddrs + Send + Sync;

    /// Asynchronously accept incoming `Connection`s
    async fn accept(&mut self) -> Result<Connection, ListenerError>;

    /// Get a slice of `Candidate`s on which to connect
    async fn candidates(&self) -> Result<&[Self::Candidate], ListenerError>;
}
