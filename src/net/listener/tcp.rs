use std::fmt;
use std::net::SocketAddr;

use super::super::Channel;
use super::Listener;
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use tokio::io::Error as TokioError;
use tokio::net::{TcpListener as TokioListener, TcpStream, ToSocketAddrs};

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
    ) -> Result<Self, <Self as Listener>::Error> {
        TokioListener::bind(candidate)
            .await
            .map_err(|e| e.into())
            .map(|listener| Self {
                listener,
                exchanger,
            })
    }
}

impl fmt::Debug for TcpListener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.listener.local_addr() {
            Ok(addr) => write!(f, "TcpListener on {}", addr),
            Err(e) => write!(f, "TcpListener errored: {}", e),
        }
    }
}

#[async_trait]
impl Listener for TcpListener {
    type Addr = SocketAddr;

    type Connection = Channel<TcpStream>;

    type Error = TokioError;

    async fn accept(&mut self) -> Result<Self::Connection, Self::Error> {
        let stream = self.listener.accept().await?.0;

        Ok(Channel::new_server(stream, self.exchanger.clone()))
    }
}
