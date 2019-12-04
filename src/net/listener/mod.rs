/// Tcp related listeners utilities
pub mod tcp;

use std::io::Error as IoError;

use super::{Connection, SecureError};
use crate as drop;

use async_trait::async_trait;

use macros::error;

use tokio::io::Error as TokioError;
use tokio::net::ToSocketAddrs;

error! {
    type: ListenerError,
    description: "error accepting incoming connection",
    causes: (TokioError, SecureError, IoError),
}

/// A trait used to accept incoming `Connection`s from other peers
#[async_trait]
pub trait Listener {
    /// The type of address that this `Listener` listens on
    type Candidate: ToSocketAddrs;

    /// Asynchronously accept incoming `Connection`s
    async fn accept(&mut self) -> Result<Connection, ListenerError>;

    /// Get a slice of `Candidate`s on which to connect
    async fn candidates(&self) -> Result<&[Self::Candidate], ListenerError>;
}
