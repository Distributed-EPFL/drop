use std::net::SocketAddr;

use super::super::{Channel, Connection};
use super::Connector;
use crate::crypto::key::exchange::{Exchanger, PublicKey};

use async_trait::async_trait;

use tokio::io::Error as TokioError;
use tokio::net::TcpStream;

/// A `Connector` that uses direct TCP connections to a remote peer
pub struct TcpDirect {}

impl Connection for TcpStream {}

#[async_trait]
impl Connector for TcpDirect {
    type Addr = SocketAddr;

    type Connection = Channel<TcpStream>;

    type Error = TokioError;

    async fn connect(
        addrs: Self::Addr,
        exchanger: Exchanger,
        pkey: &PublicKey,
    ) -> Result<Self::Connection, Self::Error> {
        let stream = TcpStream::connect(addrs).await?;

        Ok(Channel::new_client(stream, exchanger, pkey.clone()))
    }
}
