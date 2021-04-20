use std::fmt;
use std::io::ErrorKind;
use std::net::SocketAddr;

use super::super::socket::Socket;
use super::*;
use crate::crypto::key::exchange::Exchanger;
use crate::net::socket::utp::BufferedUtpStream;

use async_trait::async_trait;

use tokio::net::ToSocketAddrs;
use tokio::task;

use tracing::{debug_span, info};
use tracing_futures::Instrument;

use ::utp::UtpSocket;

/// A `Listener` that uses the micro transport protocol (μTp)
pub struct Direct {
    socket: Option<UtpSocket>,
    exchanger: Exchanger,
}

impl Direct {
    /// Create a new `UtpListener` that will be able to accept one `Connection`
    /// on the given local address.
    pub async fn new<A: ToSocketAddrs>(
        addr: A,
        exchanger: Exchanger,
    ) -> Result<Self, ListenerError> {
        Ok(Self {
            socket: Some(UtpSocket::bind(addr).await.context(Io)?),
            exchanger,
        })
    }
}

#[async_trait]
impl Listener for Direct {
    type Candidate = SocketAddr;

    async fn candidates(&self) -> Result<&[Self::Candidate], ListenerError> {
        todo!()
    }

    /// Get the local address for this `Listener`. Be aware that `UtpDirect` is
    /// a one-use `Listener` and that after accepting a `Connection` this will
    /// return an error.
    fn local_addr(&self) -> Option<SocketAddr> {
        self.socket.as_ref().map(|x| x.local_addr())
    }

    /// Accept a Utp `Connection` on this `Listener`. This `Listener` is no
    /// longer usable after succesfully accepting an incoming `Connection` and
    /// will always return an error.
    async fn establish(&mut self) -> Result<Box<dyn Socket>, ListenerError> {
        let opt: Option<UtpSocket> = self.socket.take();
        let socket: Result<UtpSocket, ListenerError> = opt
            .ok_or_else(|| ErrorKind::AddrNotAvailable.into())
            .context(Io);

        let (stream, driver) = socket?.accept().await.context(Io)?;
        let remote = stream.peer_addr();

        info!("incoming uTp connection from {}", remote);

        task::spawn(driver.instrument(debug_span!("stream_driver")));

        let buffered = BufferedUtpStream::new(stream);

        Ok(Box::new(buffered))
    }

    fn exchanger(&self) -> &Exchanger {
        &self.exchanger
    }
}

impl fmt::Display for Direct {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.socket {
            None => write!(f, "exhausted utp listener"),
            Some(ref socket) => {
                write!(f, "utp listener on {}", socket.local_addr())
            }
        }
    }
}
