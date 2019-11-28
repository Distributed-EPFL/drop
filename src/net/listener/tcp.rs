use std::net::SocketAddr;

use super::super::connector::tcp::{TcpConn, TcpError};
use super::Listener;

use async_trait::async_trait;

use tokio::net::TcpListener as TokioListener;

/// FIXME: placeholder for keypair
pub type KeyExchanger = ();

/// A plain `TcpListener` that accepts connections on a given IP address and
/// port
pub struct TcpListener {
    listener: TokioListener,
    exchanger: KeyExchanger,
}

#[async_trait]
impl Listener for TcpListener {
    type Addr = SocketAddr;

    type Connection = TcpConn;

    type Error = TcpError;

    async fn accept(&mut self) -> Result<Self::Connection, Self::Error> {
        let stream = self.listener.accept().await?;

        unimplemented!()
    }
}
