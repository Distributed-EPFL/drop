/// Listeners that use TCP as a transport protocol
mod tcp;
pub use tcp::*;

/// Listeners that use uTP as a transport protocol
mod utp;
pub use self::utp::*;

/// Directory listener
mod directory;
pub use directory::*;

use std::fmt;
use std::io::Error;
use std::net::SocketAddr;

use super::socket::Socket;
use super::{Connection, SecureError};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub enum ListenerError {
    #[snafu(display("i/o  error: {}", source))]
    #[snafu(visibility(pub))]
    Io { source: Error },

    #[snafu(display("could not secure connection: {}", source))]
    #[snafu(visibility(pub))]
    Secure { source: SecureError },
}

/// A trait used to accept incoming `Connection`s from other peers
#[async_trait]
pub trait Listener: Send + Sync {
    /// The type of address that this `Listener` listens on.
    type Candidate: Send + Sync + fmt::Display;

    /// Returns the local address on which this `Listener` listens if relevant.
    /// Typically hole punching `Listener`s will not listen on a socket and
    /// will therefore not have any local_addr
    fn local_addr(&self) -> Option<SocketAddr> {
        None
    }

    /// Accept one incoming connection while not exchanging any data
    async fn establish(&mut self) -> Result<Box<dyn Socket>, ListenerError>;

    /// Accept and secure an incoming `Connection`
    async fn accept(&mut self) -> Result<Connection, ListenerError> {
        let socket = self.establish().await?;
        let mut connection = Connection::new(socket);

        connection
            .secure_client(self.exchanger())
            .await
            .context(Secure)?;

        Ok(connection)
    }

    /// Return the `Exchanger` that should be used when securing incoming
    /// `Connection`s
    fn exchanger(&self) -> &Exchanger;

    /// Get a slice of `Candidate`s on which this `Listener` can be reached
    async fn candidates(&self) -> Result<&[Self::Candidate], ListenerError>;
}
