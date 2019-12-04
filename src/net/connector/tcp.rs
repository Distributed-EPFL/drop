use std::net::SocketAddr;

use super::super::{Socket, SocketError};
use super::{ConnectError, Connector};
use crate::crypto::key::exchange::{Exchanger, PublicKey};

use async_trait::async_trait;

use tokio::net::TcpStream;

/// A `Connector` that uses direct TCP connections to a remote peer
pub struct TcpDirect {
    exchanger: Exchanger,
}

impl TcpDirect {
    /// Create a new `TcpDirect` `Connector` using the given
    /// `Exchanger` to generate shared secrets
    pub fn new(exchanger: Exchanger) -> Self {
        Self { exchanger }
    }
}

impl Socket for TcpStream {
    fn local(&self) -> Result<SocketAddr, SocketError> {
        self.local_addr().into()
    }

    fn remote(&self) -> Result<SocketAddr, SocketError> {
        self.peer_addr().into()
    }
}

#[async_trait]
impl Connector for TcpDirect {
    type Candidate = SocketAddr;

    async fn establish(
        addrs: Self::Candidate,
        pkey: &PublicKey,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        match TcpStream::connect(addrs).await {
            Ok(stream) => Ok(Box::new(stream)),
            Err(e) => Err(ConnectError::from(e)),
        }
    }
}
