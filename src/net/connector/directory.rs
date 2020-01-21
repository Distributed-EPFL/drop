use std::io::{Error, ErrorKind};
use std::net::SocketAddr;

use super::super::common::directory::{DirectoryRequest, DirectoryResponse};
use super::super::{
    Connection, CorruptedConnection, ReceiveError, SendError, Socket,
};
use super::{ConnectError, Connector};
use crate::crypto::key::exchange::{Exchanger, PublicKey};

use async_trait::async_trait;

use tracing::{debug_span, error, info};
use tracing_futures::Instrument;

/// A `Connector` that makes use of a centralized directory in order
/// to discover peers by their `PublicKey`. This `Connector` uses `PublicKey`s
/// as `Candidate` and finds out the actual address from the directory server.
pub struct DirectoryConnector {
    /// `Connector` that will be used to open `Connection`s to peers
    connector: Box<dyn Connector<Candidate = SocketAddr>>,
    /// `Connection` to the directory server
    connection: Connection,
    exchanger: Exchanger,
}

impl DirectoryConnector {
    /// Create a new `DirectoryConnector` that will use the given `Connector` to
    /// establish connections to both the directory server and then to peers.
    ///
    /// # Arguments
    /// * `connector` - the `Connector` that will be used to establish all
    /// `Connection`s
    /// * `pkey` - the directory server's `PublicKey`
    /// * `addr` - the directory server's address
    pub async fn new(
        mut connector: Box<dyn Connector<Candidate = SocketAddr>>,
        pkey: &PublicKey,
        addr: SocketAddr,
    ) -> Result<Self, ConnectError> {
        let exchanger = connector.exchanger().clone();

        info!("connecting to directory server at {}", addr);

        let connection = connector
            .connect(pkey, &addr)
            .instrument(debug_span!("directory_connect"))
            .await?;
        info!("succesfully connected to server");

        Ok(Self {
            connector,
            connection,
            exchanger,
        })
    }

    /// Registers a local `SocketAddr` on the directory server to which this
    /// `DirectoryConnector` is connected.
    pub async fn register(
        &mut self,
        addr: SocketAddr,
    ) -> Result<(), SendError> {
        let peer_info = (*self.exchanger().keypair().public(), addr).into();

        info!("registering {} as local destination with directory", addr);

        self.connection
            .send(&DirectoryRequest::Add(peer_info))
            .await?;

        match self.connection.receive::<DirectoryResponse>().await {
            Err(e) => {
                error!("bad response from directory: {}", e);
                Err(CorruptedConnection::new().into())
            }
            Ok(DirectoryResponse::Ok) => {
                info!("registration succesfull");
                Ok(())
            }
            Ok(v) => {
                error!("unexpected answer from directory: {}", v);
                Err(CorruptedConnection::new().into())
            }
        }
    }

    /// Closes the `Connection` to the directory server. Note that calls
    /// to `Self::register` will fail after this is called.
    pub async fn close(&mut self) -> Result<(), Error> {
        self.connection.close().await
    }

    async fn handle_response(
        &mut self,
        response: Result<DirectoryResponse, ReceiveError>,
        pkey: &PublicKey,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        match response {
            Ok(DirectoryResponse::Found(s_addr)) => {
                info!("peer {} is at {}", pkey, s_addr);
                self.connector.establish(&s_addr).await
            }
            Ok(DirectoryResponse::NotFound(pkey)) => {
                error!("directory server does not know peer {}", pkey);
                Err(Error::from(ErrorKind::AddrNotAvailable).into())
            }
            Ok(_) => {
                error!("invalid response from directory server");
                Err(Error::from(ErrorKind::AddrNotAvailable).into())
            }
            Err(e) => {
                error!("error reading response from directory: {}", e);
                Err(Error::from(ErrorKind::BrokenPipe).into())
            }
        }
    }
}

#[async_trait]
impl Connector for DirectoryConnector {
    type Candidate = PublicKey;

    fn exchanger(&self) -> &Exchanger {
        &self.exchanger
    }

    /// Open a `Socket` to a peer using its `PublicKey` to find its `SocketAddr`
    /// from the directory server.
    async fn establish(
        &mut self,
        pkey: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        info!("finding peer address for public key {}", pkey);

        let req = DirectoryRequest::Fetch(*pkey);

        if let Err(e) = self.connection.send(&req).await {
            error!("directory server is unavailable: {}", e);
            return Err(Error::from(ErrorKind::AddrNotAvailable).into());
        }

        let resp = self.connection.receive::<DirectoryResponse>().await;

        self.handle_response(resp, pkey).await
    }
}

#[cfg(test)]
mod test {
    use std::net::Ipv4Addr;

    use super::*;
    use crate::net::connector::tcp::TcpDirect;
    use crate::net::listener::tcp::TcpListener;
    use crate::net::listener::{DirectoryListener, Listener};

    use tokio::task;

    #[tokio::test]
    async fn directory_connect() {
        let addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, 0).into();
        let dir_exchanger = Exchanger::random();
        let dir_public = *dir_exchanger.keypair().public();
        let mut listener = DirectoryListener::new(addr, dir_exchanger.clone())
            .await
            .expect("bind failed");

        let dir_addr = listener.local_addr().unwrap();

        task::spawn(async move {
            for _ in 0..2usize {
                assert!(
                    listener.accept().await.is_ok(),
                    "failed to server directory request"
                );
            }
        });

        let peer_exchanger = Exchanger::random();
        let peer_public = *peer_exchanger.keypair().public();

        let mut listener = TcpListener::new(addr, peer_exchanger.clone())
            .await
            .expect("failed to listen");

        let peer_addr = listener.local_addr().unwrap();

        let handle = task::spawn(async move {
            let connector = Box::new(TcpDirect::new(Exchanger::random()));
            let mut connector =
                DirectoryConnector::new(connector, &peer_public, dir_addr)
                    .await
                    .expect("connect to directory failed");

            connector
                .connect(&peer_public, &peer_public)
                .await
                .expect("failed to connect to peer");
        });
        let mut tcp = Box::new(TcpDirect::new(peer_exchanger));

        let mut dir_conn = tcp
            .connect(&dir_public, &dir_addr)
            .await
            .expect("failed to connect to directory");

        dir_conn
            .send(&DirectoryRequest::Add((peer_public, peer_addr).into()))
            .await
            .expect("failed to register with directory");

        dir_conn
            .send(&0u32)
            .await
            .expect("failed to close directory connection");

        listener.accept().await.expect("failed to accept");

        handle.await.expect("remote peer failed");
    }
}
