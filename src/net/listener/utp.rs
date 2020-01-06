use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use super::super::Connection;
use super::{Listener, ListenerError};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use tokio::net::ToSocketAddrs;
use tokio::task;

use utp::UtpSocket;

/// A listener using the micro transport protocol (uTp)
pub struct UtpListener {
    socket: Option<UtpSocket>,
    exchanger: Exchanger,
}

impl UtpListener {
    /// Create a new `UtpListener` that will be able to accept one `Connection`
    /// on the given local address.
    pub async fn new<A: ToSocketAddrs>(
        addr: A,
        exchanger: Exchanger,
    ) -> Result<Self, ListenerError> {
        Ok(Self {
            socket: Some(UtpSocket::bind(addr).await?),
            exchanger,
        })
    }
}

#[async_trait]
impl Listener for UtpListener {
    type Candidate = SocketAddr;

    async fn candidates(&self) -> Result<&[Self::Candidate], ListenerError> {
        todo!()
    }

    /// Get the local address for this `Listener`. Be aware that `UtpDirect` is
    /// a one-use `Listener` and that after accepting a `Connection` this will
    /// return an error.
    fn local_addr(&self) -> Result<SocketAddr, ListenerError> {
        self.socket
            .as_ref()
            .ok_or_else(|| {
                let io: Error = ErrorKind::AddrNotAvailable.into();

                io.into()
            })
            .map(|x| x.local_addr())
    }

    /// Accept a Utp `Connection` on this `Listener`. This `Listener` is no
    /// longer usable after succesfully accepting an incoming `Connection` and
    /// will always return an error.
    async fn accept(&mut self) -> Result<Connection, ListenerError> {
        let opt: Option<UtpSocket> = self.socket.take();
        let socket: Result<UtpSocket, ListenerError> = opt.ok_or_else(|| {
            let io: Error = ErrorKind::AddrNotAvailable.into();
            io.into()
        });

        let (stream, driver) = socket?.accept().await?;

        task::spawn(driver);
        let mut connection = Connection::new(Box::new(stream));

        connection.secure_client(&self.exchanger).await?;

        Ok(connection)
    }
}
