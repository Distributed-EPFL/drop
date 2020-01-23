use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

use super::super::socket::utp::BufferedUtpStream;
use super::{ConnectError, Connector, Socket};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use utp::{UtpSocket, UtpStream};

use tokio::task;

use tracing::{debug_span, info};
use tracing_futures::Instrument;

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
        &mut self,
        candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        let local: SocketAddr = match *candidate {
            SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
            SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
        };
        let socket = UtpSocket::bind(local).await?;

        info!(
            "connecting {} -> {} using uTp",
            socket.local_addr(),
            candidate
        );

        let (stream, driver) = socket.connect(*candidate).await?;

        info!("connection to {} established", candidate);

        task::spawn(driver.instrument(debug_span!("stream_driver")));

        Ok(Box::new(BufferedUtpStream::new(stream)))
    }

    fn exchanger(&self) -> &Exchanger {
        &self.exchanger
    }
}

impl Socket for UtpStream {
    fn remote(&self) -> std::io::Result<SocketAddr> {
        Ok(self.peer_addr())
    }

    fn local(&self) -> std::io::Result<SocketAddr> {
        Ok(self.local_addr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::listener::{Listener, UtpListener};
    use crate::test::*;

    #[tokio::test]
    async fn utp_correct() {
        init_logger();

        let addr = next_test_ip4();
        let server = Exchanger::random();
        let client = Exchanger::random();
        let mut utp = UtpListener::new(addr, server.clone())
            .instrument(debug_span!("bind"))
            .await
            .expect("failed to bind local");

        task::spawn(
            async move {
                let mut utp = UtpDirect::new(client);
                let mut connection = utp
                    .connect(server.keypair().public(), &addr)
                    .instrument(debug_span!("connect"))
                    .await
                    .expect("failed to connect");

                connection
                    .send(&0u32)
                    .instrument(debug_span!("send"))
                    .await
                    .expect("failed to send");

                connection
                    .close()
                    .instrument(debug_span!("close"))
                    .await
                    .expect("failed to close");
            }
            .instrument(debug_span!("client")),
        );

        let mut connection = utp
            .accept()
            .instrument(debug_span!("accept"))
            .await
            .expect("failed to accept connection");

        let data = connection
            .receive::<u32>()
            .instrument(debug_span!("server_receive"))
            .await
            .expect("failed to receive");

        assert_eq!(data, 0u32, "wrong data received");

        connection
            .flush()
            .instrument(debug_span!("server_flush"))
            .await
            .expect("failed to flush");
        connection
            .close()
            .instrument(debug_span!("server_close"))
            .await
            .expect("failed to close");
    }
}
