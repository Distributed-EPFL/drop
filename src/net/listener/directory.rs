use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;

use super::super::common::directory::{
    DirectoryPeer, DirectoryRequest, DirectoryResponse,
};
use super::super::Connection;
use super::tcp::TcpListener;
use super::{Listener, ListenerError};
use crate::crypto::key::exchange::{Exchanger, PublicKey};

use async_trait::async_trait;

use tokio::net::ToSocketAddrs;

use tracing::{error, info};

/// A `Listener` that is used to keep track of peers in the system.
/// It works as a directory server where each peer registers with the address
/// it is reachable at and the public key that should be used when communicating
/// with that destination.
/// Other peers can then ask this directory where to reach a given public key.
pub struct DirectoryListener {
    listener: TcpListener,
    peers: HashMap<PublicKey, DirectoryPeer>,
}

impl DirectoryListener {
    /// Create a new `DirectoryListener` that will serve requests on
    /// the given address.
    ///
    /// # Example
    /// ```
    /// # use std::net::SocketAddr;
    /// use drop::crypto::key::exchange::Exchanger;
    /// # use drop::net::listener::ListenerError;
    /// use drop::net::listener::DirectoryListener;
    ///
    /// # async fn doc() -> Result<(), ListenerError> {
    /// let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    /// let mut listener = DirectoryListener::new(addr, Exchanger::random()).await?;
    /// # Ok(()) }
    /// ```
    pub async fn new<A: ToSocketAddrs + fmt::Display>(
        local: A,
        exchanger: Exchanger,
    ) -> Result<Self, ListenerError> {
        Ok(Self {
            listener: TcpListener::new(local, exchanger).await?,
            peers: HashMap::new(),
        })
    }

    /// Serve directory requests using this `DirectoryListener`.
    /// # Example
    /// ```
    /// # use std::net::{Ipv4Addr, SocketAddr};
    ///
    /// use drop::crypto::key::exchange::Exchanger;
    /// use drop::net::listener::{DirectoryListener, ListenerError};
    ///
    /// # async fn doc() -> Result<(), ListenerError> {
    /// let local: SocketAddr = (Ipv4Addr::UNSPECIFIED, 0).into();
    /// let exchanger = Exchanger::random();
    /// let mut listener = DirectoryListener::new(local, exchanger).await?;
    ///
    /// tokio::task::spawn(async move { listener.serve().await });
    /// # Ok(())
    /// # }
    /// ```
    pub async fn serve(&mut self) -> Result<(), ListenerError> {
        loop {
            let mut connection = match self.accept().await {
                Err(e) => {
                    error!("failed to accept connection: {}", e);
                    break;
                }
                Ok(c) => c,
            };

            info!("directory connection from {}", connection.peer_addr()?);

            loop {
                let request =
                    match connection.receive::<DirectoryRequest>().await {
                        Ok(req) => req,
                        Err(e) => {
                            error!("failed to read request: {}", e);
                            break;
                        }
                    };

                let response = match request {
                    DirectoryRequest::Add(peer_info) => {
                        let addr = peer_info.addr();

                        self.peers.insert(*peer_info.public(), peer_info);

                        info!("added {} to known peers", addr);

                        DirectoryResponse::Ok
                    }
                    DirectoryRequest::Fetch(pkey) => {
                        match self.peers.entry(pkey) {
                            Entry::Vacant(_) => {
                                info!("peer {} not found", pkey);
                                DirectoryResponse::NotFound(pkey)
                            }
                            Entry::Occupied(e) => {
                                let addr = e.get().addr();
                                info!("found request peer at {}", addr);
                                DirectoryResponse::Found(addr)
                            }
                        }
                    }
                };

                if connection.send(&response).await.is_err() {
                    error!(
                        "failed to respond to request from {}",
                        connection.peer_addr().unwrap()
                    );
                    break;
                }
            }

            info!("done handling request from {}", connection.peer_addr()?);
        }

        Ok(())
    }
}

#[async_trait]
impl Listener for DirectoryListener {
    type Candidate = SocketAddr;

    /// Accept a new `Connection` on this `Listener`. `DirectoryListener` is a
    /// bit special as it first answers any number of directory request once
    /// a `Connection` has been established before handing the `Connection`
    /// over.
    async fn accept(&mut self) -> Result<Connection, ListenerError> {
        let connection = self.listener.accept().await?;

        Ok(connection)
    }

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr()
    }

    async fn candidates(&self) -> Result<&[Self::Candidate], ListenerError> {
        todo!()
    }
}

impl fmt::Display for DirectoryListener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "directory listener at {} knows {} peers",
            self.local_addr().unwrap(),
            self.peers.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use super::*;
    use crate::net::connector::TcpDirect;
    use crate::net::connector::{Connector, DirectoryConnector};
    use crate::test::init_logger;

    use tokio::task;

    use tracing::debug_span;
    use tracing_futures::Instrument;

    #[tokio::test]
    async fn add_peer() {
        init_logger();

        let dir_exchanger = Exchanger::random();
        let server_addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, 0).into();

        let mut listener =
            DirectoryListener::new(server_addr, dir_exchanger.clone())
                .await
                .expect("bind failed");
        let server_addr = listener.local_addr().expect("get addr failed");

        task::spawn(
            async move {
                listener.serve().await.expect("failed to serve");
                assert_eq!(listener.peers.len(), 1, "peer was not added");
            }
            .instrument(debug_span!("directory_server")),
        );

        let dir_pub = *dir_exchanger.keypair().public();

        let peer_addr: SocketAddr = "127.0.0.1:9090".parse().unwrap();
        let peer_exchanger = Exchanger::random();
        let connector = Box::new(TcpDirect::new(peer_exchanger.clone()));
        let peer_key = *peer_exchanger.keypair().public();

        let handle = task::spawn(
            async move {
                let tcp = Box::new(TcpDirect::new(Exchanger::random()));
                let mut connector =
                    DirectoryConnector::new(tcp, &dir_pub, server_addr)
                        .instrument(debug_span!("directory_client"))
                        .await
                        .expect("directory connect failed");

                connector
                    .connect(&peer_key, &peer_key)
                    .instrument(debug_span!("connect"))
                    .await
                    .expect("peer connection failed");
            }
            .instrument(debug_span!("peer_finder")),
        );

        let mut listener = TcpListener::new(peer_addr, peer_exchanger)
            .await
            .expect("failed to listen");

        let mut connector =
            DirectoryConnector::new(connector, &dir_pub, server_addr)
                .instrument(debug_span!("peer_to_find"))
                .await
                .expect("failed to connect to directory");

        connector
            .register(peer_addr)
            .instrument(debug_span!("register_peer"))
            .await
            .expect("failed to register peer");

        connector
            .close()
            .await
            .expect("failed to close directory connection");

        listener
            .accept()
            .instrument(debug_span!("accept"))
            .await
            .expect("failed to accept");

        handle.await.expect("peer failed to connect");
    }
}
