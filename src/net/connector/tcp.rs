use std::net::SocketAddr;

use super::super::Socket;
use super::{ConnectError, Connector};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use tokio::io::Error as TokioError;
use tokio::net::TcpStream;

/// A `Connector` that uses direct TCP connections to a remote peer
pub struct TcpDirect {
    exchanger: Exchanger,
}

impl TcpDirect {
    /// Create a new `TcpDirect` `Connector` using the given
    /// `Exchanger` to compute shared secrets
    pub fn new(exchanger: Exchanger) -> Self {
        Self { exchanger }
    }
}

impl Socket for TcpStream {
    fn local(&self) -> Result<SocketAddr, TokioError> {
        self.local_addr()
    }

    fn remote(&self) -> Result<SocketAddr, TokioError> {
        self.peer_addr()
    }
}

#[async_trait]
impl Connector for TcpDirect {
    /// This `Connector` uses a pair of `IpAddr` and port as destination
    type Candidate = SocketAddr;

    /// Returns the local client's `Exchanger`
    fn exchanger(&self) -> &Exchanger {
        &self.exchanger
    }

    /// Open a `Socket` to the specified destination using TCP
    async fn establish(
        candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        let stream: Box<dyn Socket> =
            Box::new(TcpStream::connect(candidate).await?);

        Ok(stream)
    }
}
