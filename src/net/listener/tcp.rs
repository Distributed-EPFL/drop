use std::fmt;
use std::net::SocketAddr;

use super::super::Connection;
use super::{Listener, ListenerError};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use tokio::net::{TcpListener as TokioListener, ToSocketAddrs};

/// A plain `TcpListener` that accepts connections on a given IP address and
/// port
pub struct TcpListener {
    listener: TokioListener,
    exchanger: Exchanger,
}

impl TcpListener {
    /// Create a new `TcpListener` that will listen on the candidate address
    pub async fn new<A: ToSocketAddrs>(
        candidate: A,
        exchanger: Exchanger,
    ) -> Result<Self, ListenerError> {
        TokioListener::bind(candidate)
            .await
            .map(|listener| Self {
                listener,
                exchanger,
            })
            .map_err(|e| e.into())
    }
}

impl fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.listener.local_addr() {
            Ok(addr) => write!(f, "tcp listener on {}", addr),
            Err(e) => write!(f, "tcp listener errored: {}", e),
        }
    }
}

#[async_trait]
impl Listener for TcpListener {
    type Candidate = SocketAddr;

    async fn candidates(&self) -> Result<&[Self::Candidate], ListenerError> {
        unimplemented!()
    }

    /// Accept an incoming `Connection` from this `TcpListener` and performs
    /// key exchange to authenticate the remote peer.
    async fn accept(&mut self) -> Result<Connection, ListenerError> {
        let stream = self.listener.accept().await.map(|(stream, _)| stream)?;
        let mut connection = Connection::new(Box::new(stream));

        connection.secure_client(&self.exchanger).await?;
        Ok(connection)
    }
}
