use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr};

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
