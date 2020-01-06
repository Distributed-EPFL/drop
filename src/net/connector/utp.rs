use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use super::{ConnectError, Connector, Socket};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use utp::{UtpSocket, UtpStream};

use tokio::task;

/// A `Connector` using the micro transport protocol
pub struct UtpDirect {
    exchanger: Exchanger,
}

impl UtpDirect {
    pub fn new(exchanger: Exchanger) -> Self {
        Self { exchanger }
    }
}

#[async_trait]
impl Connector for UtpDirect {
    type Candidate = SocketAddr;

    async fn establish(
        candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        let local = match *candidate {
            SocketAddr::V4(_) => {
                SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
            }
            SocketAddr::V6(_) => SocketAddr::V6(SocketAddrV6::new(
                Ipv6Addr::UNSPECIFIED,
                0,
                0,
                0,
            )),
        };
        let socket = UtpSocket::bind(local).await?;
        let (stream, driver) = socket.connect(*candidate).await?;

        task::spawn(driver);

        Ok(Box::new(stream))
    }

    fn exchanger(&self) -> &Exchanger {
        &self.exchanger
    }
}

impl Socket for UtpStream {
    fn remote(&self) -> std::io::Result<SocketAddr> {
        todo!()
    }

    fn local(&self) -> std::io::Result<SocketAddr> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};
    use std::sync::atomic::{AtomicU16, Ordering};

    use super::*;
    use crate::net::listener::{utp::UtpListener, Listener};

    fn next_test_port() -> u16 {
        static PORT_OFFSET: AtomicU16 = AtomicU16::new(0);
        const PORT_START: u16 = 9600;

        PORT_START + PORT_OFFSET.fetch_add(1, Ordering::Relaxed)
    }

    fn next_test_ip4() -> SocketAddr {
        SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(127, 0, 0, 1),
            next_test_port(),
        ))
    }

    #[tokio::test]
    async fn utp_correct() {
        let addr = next_test_ip4();
        let server = Exchanger::random();
        let client = Exchanger::random();
        let mut utp = UtpListener::new(addr, server.clone())
            .await
            .expect("failed to bind local");

        task::spawn(async move {
            let utp = UtpDirect::new(client);
            let mut connection = utp
                .connect(server.keypair().public(), &addr)
                .await
                .expect("failed to connect");

            connection.send(&0u32).await.expect("failed to send");
            connection.close().await.expect("failed to close");
        });

        let mut connection =
            utp.accept().await.expect("failed to accept connection");

        let data = connection
            .receive::<u32>()
            .await
            .expect("failed to receive");

        assert_eq!(data, 0u32, "wrong data received");

        connection.close().await.expect("faield to close");
    }
}
