use crate::{
    crypto::key::exchange::Exchanger,
    net::{
        listen::errors::{ListenerError, Secure},
        socket::Socket,
        Connection,
    },
};

use async_trait::async_trait;

use snafu::ResultExt;

use std::fmt;
use std::net::SocketAddr;

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
    async fn candidates(&self) -> Result<Vec<Self::Candidate>, ListenerError>;
}
