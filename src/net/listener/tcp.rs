use std::fmt;
use std::io::Error as IoError;

use super::super::{Connection, SecureError};
use super::Listener;
use crate as drop;
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use macros::error;

use tokio::io::Error as TokioError;
use tokio::net::{TcpListener as TokioListener, ToSocketAddrs};

error! {
    type: ListenerError,
    description: "incoming connection error",
    causes: (TokioError, IoError, SecureError)
}

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
            Ok(addr) => write!(f, "tcp listener on {}", addr),
            Err(e) => write!(f, "tcp listener errored: {}", e),
        }
    }
}

#[async_trait]
impl Listener for TcpListener {
    async fn accept(&mut self) -> Result<Connection, ListenerError> {
        let stream = Box::new(self.listener.accept().await?.0);
        let mut connection = Connection::new(stream);

        connection.secure_client(&self.exchanger).await?;
        Ok(connection)
    }
}
