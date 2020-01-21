/// Listeners that use TCP as a transport protocol
pub mod tcp;

/// Listeners that use uTP as a transport protocol
pub mod utp;

mod directory;

pub use directory::*;

use std::fmt;
use std::net::SocketAddr;

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
    /// The type of address that this `Listener` listens on.
    type Candidate: ToSocketAddrs + Send + Sync + fmt::Display;

    /// Returns the local address on which this `Listener` listens if relevant.
    /// Typically hole punching `Listener`s will not listen on a socket and
    /// will therefore not have any local_addr
    fn local_addr(&self) -> Option<SocketAddr> {
        None
    }

    /// Asynchronously accept one incoming `Connection`
    async fn accept(&mut self) -> Result<Connection, ListenerError>;

    /// Get a slice of `Candidate`s on which to this `Listener` can be reached
    async fn candidates(&self) -> Result<&[Self::Candidate], ListenerError>;
}
