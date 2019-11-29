use std::net::SocketAddr;

use super::Listener;
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use tokio::io::Error as TokioError;
use tokio::net::{TcpListener as TokioListener, TcpStream};

/// A plain `TcpListener` that accepts connections on a given IP address and
/// port
pub struct TcpListener {
    listener: TokioListener,
    exchanger: Exchanger,
}

#[async_trait]
impl Listener for TcpListener {
    type Addr = SocketAddr;

    type Connection = TcpStream;

    type Error = TokioError;

    async fn accept(&mut self) -> Result<Self::Connection, Self::Error> {
        let stream = self.listener.accept().await?;

        unimplemented!()
    }
}
