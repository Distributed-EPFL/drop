use std::fmt;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use super::super::Connection;
use super::{Listener, ListenerError};
use crate::crypto::key::exchange::Exchanger;
use crate::net::socket::utp::BufferedUtpStream;

use async_trait::async_trait;

use tokio::net::ToSocketAddrs;
use tokio::task;

use tracing::{debug_span, info};
use tracing_futures::Instrument;

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
    fn local_addr(&self) -> Option<SocketAddr> {
        self.socket.as_ref().map(|x| x.local_addr())
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
        let remote = stream.peer_addr();

        info!("incoming uTp connection from {}", remote);

        task::spawn(driver.instrument(debug_span!("stream_driver")));

        let buffered = BufferedUtpStream::new(stream);

        let mut connection = Connection::new(Box::new(buffered));

        connection
            .secure_client(&self.exchanger)
            .instrument(debug_span!("key_exchange"))
            .await?;

        info!("accepted secure connection from {}", remote);

        Ok(connection)
    }
}

impl fmt::Display for UtpListener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.socket {
            None => write!(f, "exhausted utp listener"),
            Some(ref socket) => {
                write!(f, "utp listener on {}", socket.local_addr())
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::net::connector::{Connector, UtpDirect};
    use crate::test::*;
    use crate::{exchange_data_and_compare, generate_connection};

    #[tokio::test]
    async fn utp_listener_fmt() {
        let addr = next_test_ip4();
        let exchanger = Exchanger::random();
        let listener = UtpListener::new(addr, exchanger)
            .await
            .expect("bind failed");

        assert_eq!(
            format!("{}", listener),
            format!("utp listener on {}", listener.local_addr().unwrap()),
        );
    }

    #[tokio::test]
    async fn utp_double_accept() {
        init_logger();

        let addr = next_test_ip4();
        let exchanger = Exchanger::random();
        let mut listener = UtpListener::new(addr, exchanger.clone())
            .await
            .expect("bind failed");
        let addr = listener.local_addr().unwrap();

        let handle = task::spawn(async move {
            let exch = Exchanger::random();
            let mut connector = UtpDirect::new(exch);
            let mut connection = connector
                .connect(exchanger.keypair().public(), &addr)
                .await
                .expect("connect failed");

            connection.close().await.expect("close failed");
        });

        let mut connection = listener.accept().await.expect("accept failed");

        listener
            .accept()
            .await
            .expect_err("second accept succeeded");

        connection.close().await.expect("close failed");

        handle.await.expect("connector failed");
    }

    pub async fn setup_utp() -> (Connection, Connection) {
        generate_connection!(UtpListener, UtpDirect);
    }
}
