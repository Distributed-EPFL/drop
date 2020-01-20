use std::net::SocketAddr;

use super::super::Socket;
use super::{ConnectError, Connector};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use tokio::net::TcpStream;

use tracing::info;

/// A `Connector` that uses direct TCP connections to a remote peer
pub struct TcpDirect {
    exchanger: Exchanger,
}

impl TcpDirect {
    /// Create a new `TcpDirect` `Connector` using the given
    /// `Exchanger` to compute shared secrets
    ///
    /// # Arguments
    /// * `exchanger` - The key exchanger to be used when handshaking with
    /// remote peers
    pub fn new(exchanger: Exchanger) -> Self {
        Self { exchanger }
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
        info!("establishing tcp connection to {}", candidate);

        let stream: Box<dyn Socket> =
            Box::new(TcpStream::connect(candidate).await?);

        Ok(stream)
    }
}
