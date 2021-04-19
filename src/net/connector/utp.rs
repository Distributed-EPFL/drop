use std::fmt;
use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, UdpSocket};

use super::super::socket::utp::BufferedUtpStream;
use super::*;
use crate::crypto::key::exchange::{Exchanger, PublicKey};

use async_trait::async_trait;

use tokio::task;

use tracing::{debug_span, info};
use tracing_futures::Instrument;

use ::utp::UtpSocket;

/// A `Connector` using the micro transport protocol
pub struct Direct {
    exchanger: Exchanger,
}

impl Direct {
    /// Create a new [`Direct`] muTP [`Connector`]
    ///
    /// [`Connector`]: super::Connector
    pub fn new(exchanger: Exchanger) -> Self {
        Self { exchanger }
    }
}

#[async_trait]
impl Connector for Direct {
    type Candidate = SocketAddr;

    async fn establish(
        &self,
        _: &PublicKey,
        candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        let local: SocketAddr = match *candidate {
            SocketAddr::V4(_) => (Ipv4Addr::UNSPECIFIED, 0).into(),
            SocketAddr::V6(_) => (Ipv6Addr::UNSPECIFIED, 0).into(),
        };

        let socket = UtpSocket::bind(local).await.context(Io)?;

        info!(
            "connecting {} -> {} using uTp",
            socket.local_addr(),
            candidate
        );

        let (stream, driver) = socket.connect(*candidate).await.context(Io)?;

        info!("connection to {} established", candidate);

        task::spawn(driver.instrument(debug_span!("stream_driver")));

        Ok(Box::new(BufferedUtpStream::new(stream)))
    }

    fn exchanger(&self) -> &Exchanger {
        &self.exchanger
    }
}

pub struct Wrap {
    exchanger: Exchanger,
}

#[async_trait]
impl Connector for Wrap {
    /// The `Candidate` is a localy bound [`UdpSocket`] and a destination address.
    /// This is mostly useful when punching holes through NATs since we can
    /// re-use the socket used to punch the hole to establish a [`Connection`].
    type Candidate = Info;

    async fn establish(
        &self,
        _: &PublicKey,
        _candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        todo!()
    }

    fn exchanger(&self) -> &Exchanger {
        &self.exchanger
    }
}

/// `Candidate` used for the [`Wrap`] uTP connector
pub struct Info(UdpSocket, SocketAddr);

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} -> {}",
            self.0.local_addr().map_or_else(|_| Err(fmt::Error), Ok)?,
            self.1
        )
    }
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//     use crate::net::listener::Listener;
//     use crate::net::{UtpConnector, UtpListener};
//     use crate::test::*;

//     #[tokio::test]
//     async fn utp_correct() {
//         init_logger();

//         let addr = next_test_ip4();
//         let server = Exchanger::random();
//         let client = Exchanger::random();
//         let mut utp = UtpListener::new(addr, server.clone())
//             .instrument(debug_span!("bind"))
//             .await
//             .expect("failed to bind local");

//         task::spawn(
//             async move {
//                 let utp = UtpConnector::new(client);
//                 let mut connection = utp
//                     .connect(server.keypair().public(), &addr)
//                     .instrument(debug_span!("connect"))
//                     .await
//                     .expect("failed to connect");

//                 connection
//                     .send(&0u32)
//                     .instrument(debug_span!("send"))
//                     .await
//                     .expect("failed to send");

//                 connection
//                     .close()
//                     .instrument(debug_span!("close"))
//                     .await
//                     .expect("failed to close");
//             }
//             .instrument(debug_span!("client")),
//         );

//         let mut connection = utp
//             .accept()
//             .instrument(debug_span!("accept"))
//             .await
//             .expect("failed to accept connection");

//         let data = connection
//             .receive::<u32>()
//             .instrument(debug_span!("server_receive"))
//             .await
//             .expect("failed to receive");

//         assert_eq!(data, 0u32, "wrong data received");

//         connection
//             .flush()
//             .instrument(debug_span!("server_flush"))
//             .await
//             .expect("failed to flush");
//         connection
//             .close()
//             .instrument(debug_span!("server_close"))
//             .await
//             .expect("failed to close");
//     }
// }
