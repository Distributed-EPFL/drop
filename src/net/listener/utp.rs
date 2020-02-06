use std::fmt;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use super::super::socket::Socket;
use super::{Listener, ListenerError};
use crate::crypto::key::exchange::Exchanger;
use crate::net::socket::utp::BufferedUtpStream;

use async_trait::async_trait;

use tokio::net::ToSocketAddrs;
use tokio::task;

use tracing::{debug_span, info};
use tracing_futures::Instrument;

use utp::UtpSocket;

/// A `Listener` that uses the micro transport protocol (μTp)
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
    async fn establish(&mut self) -> Result<Box<dyn Socket>, ListenerError> {
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

        Ok(Box::new(buffered))
    }

    fn exchanger(&self) -> &Exchanger {
        &self.exchanger
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
    use crate::net::Connection;
    use crate::test::*;
    use crate::{exchange_data_and_compare, generate_connection};

    use serde::{Deserialize, Serialize};

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

    #[tokio::test]
    async fn utp_u8_exchange() {
        crate::test::init_logger();
        exchange_data_and_compare!(0, u8, setup_utp);
    }

    #[tokio::test]
    async fn utp_u16_exchange() {
        exchange_data_and_compare!(0, u16, setup_utp);
    }

    #[tokio::test]
    async fn utp_u32_exchange() {
        exchange_data_and_compare!(0, u32, setup_utp);
    }

    #[tokio::test]
    async fn utp_u64_exchange() {
        exchange_data_and_compare!(0, u64, setup_utp);
    }

    #[tokio::test]
    async fn utp_i8_exchange() {
        exchange_data_and_compare!(0, i8, setup_utp);
    }

    #[tokio::test]
    async fn utp_i16_exchange() {
        exchange_data_and_compare!(0, i16, setup_utp);
    }

    #[tokio::test]
    async fn utp_i32_exchange() {
        exchange_data_and_compare!(0, i32, setup_utp);
    }

    #[tokio::test]
    async fn utp_i64_exchange() {
        exchange_data_and_compare!(0, i64, setup_utp);
    }

    #[tokio::test]
    async fn utp_struct_exchange() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct T {
            a: u32,
            b: u64,
            c: A,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct A {
            a: u8,
            b: u16,
        }

        let data = T {
            a: 258,
            b: 30567,
            c: A { a: 66, b: 245 },
        };

        exchange_data_and_compare!(data, T, setup_utp);
    }

    #[ignore] // until tokio-utp crate is fixed
    #[tokio::test]
    async fn utp_hashmap_exchange() {
        use std::collections::HashMap;

        let mut hashmap: HashMap<u32, u128> = HashMap::default();

        for _ in 0..rand::random::<usize>() % 2048 {
            hashmap.insert(rand::random(), rand::random());
        }

        exchange_data_and_compare!(hashmap, HashMap<u32, u128>, setup_utp);
    }
}
