use std::collections::HashMap;
use std::io::{Error as IoError, ErrorKind};
use std::net::SocketAddr;

use super::super::common::directory::{Info, Request, Response};
use super::super::{
    Connection, CorruptedConnection, ReceiveError, SendError, Socket,
};
use super::{ConnectError, Connector};
use crate as drop;
use crate::crypto::key::exchange::{Exchanger, PublicKey};
use crate::error::Error;

use async_trait::async_trait;

use macros::error;

use tracing::{debug, error as log_error, info, trace_span};
use tracing_futures::Instrument;

error! {
    type: DirectoryError,
    description: "directory server failure",
    causes: (ConnectError, IoError, SendError, ReceiveError)
}

/// A `Connector` that makes use of a centralized directory in order
/// to discover peers by their `PublicKey`. This `Connector` uses `PublicKey`s
/// as `Candidate` and finds out the actual address from the directory server.
pub struct DirectoryConnector {
    /// `Connector` that will be used to open `Connection`s to peers
    connector: Box<dyn Connector<Candidate = SocketAddr>>,
    /// Previously established connections to directories
    connections: HashMap<SocketAddr, Connection>,
    /// Local cache of mappings between `PublicKey`s and peer addresses
    peer_cache: HashMap<PublicKey, SocketAddr>,
}

impl DirectoryConnector {
    /// Create a new `DirectoryConnector` that will use the given `Connector` to
    /// establish connections to both the directory server and then to peers.
    ///
    /// # Arguments
    /// * `connector` - the `Connector` that will be used to establish all
    /// `Connection`s including the `Connection` to the directory server
    pub fn new(connector: Box<dyn Connector<Candidate = SocketAddr>>) -> Self {
        Self {
            connector,
            connections: HashMap::new(),
            peer_cache: HashMap::new(),
        }
    }

    /// Registers a local `SocketAddr` on the directory server that can be
    /// reached by other peers. This will use the provided `Exchanger` as the
    /// `PublicKey` when registering.
    ///
    /// # Arguments
    /// * `dir_addr`: Address of the directory server to register with
    /// * `local_addr`: Local address to register with the directory server
    pub async fn register(
        &mut self,
        dir_addr: SocketAddr,
        local_addr: SocketAddr,
    ) -> Result<(), SendError> {
        let peer_info =
            (*self.exchanger().keypair().public(), local_addr).into();

        info!(
            "registering {} as local destination with directory at {}",
            local_addr, dir_addr
        );

        let public = *self.connector.exchanger().keypair().public();
        let connection = self
            .find_directory_server(&public, &dir_addr)
            .await
            .map_err(|e| {
            IoError::new(ErrorKind::NotConnected, format!("{}", e))
        })?;

        connection.send(&Request::Add(peer_info)).await?;

        match connection.receive::<Response>().await {
            Err(e) => {
                log_error!("bad response from directory: {}", e);
                Err(CorruptedConnection::new().into())
            }
            Ok(Response::Ok) => {
                info!("registration succesfull");
                Ok(())
            }
            Ok(v) => {
                log_error!("unexpected answer from directory: {}", v);
                Err(CorruptedConnection::new().into())
            }
        }
    }

    /// Use this `DirectoryConnector` as a barrier. This method will wait until
    /// the specified `DirectoryServer` knows the address of `nr_peer` peers
    /// before returning, ensuring that the system in a usable state before
    /// continuing.
    ///
    /// # Arguments
    /// * `nr_peer` The number of peers to wait before returning
    /// * `dir_addr` The address of the directory server to contact
    /// * `pkey` The directory server's `PublicKey`
    pub async fn wait(
        &mut self,
        nr_peer: usize,
        dir_addr: SocketAddr,
        pkey: &PublicKey,
    ) -> Result<Vec<Info>, DirectoryError> {
        let connection = self.find_directory_server(pkey, &dir_addr).await?;
        let mut peers = Vec::with_capacity(nr_peer);

        connection.send_plain(&Request::Wait(nr_peer)).await?;

        debug!("waiting for {} peers in the directory", nr_peer);

        for _ in 0..nr_peer {
            let peer = connection.receive_plain::<Info>().await?;
            debug!("got {} from directory", peer);
            peers.push(peer);
        }

        info!("got {} peers from directory", nr_peer);
        Ok(peers)
    }

    async fn find_directory_server(
        &mut self,
        pkey: &PublicKey,
        dir_addr: &SocketAddr,
    ) -> Result<&mut Connection, ConnectError> {
        // `Entry` API does not support async so really no way to avoid double
        // lookup...
        if !self.connections.contains_key(dir_addr) {
            let socket = self.connector.establish(pkey, dir_addr).await?;

            self.connections.insert(*dir_addr, Connection::new(socket));
        }

        self.connections
            .get_mut(dir_addr)
            .ok_or_else(|| IoError::new(ErrorKind::NotConnected, "").into())
    }

    /// Closes the `Connection`s to every directory server
    pub async fn close(&mut self) -> Result<(), IoError> {
        for c in self.connections.values_mut() {
            c.close().await?;
        }
        Ok(())
    }

    async fn handle_response(
        &mut self,
        response: Result<Response, ReceiveError>,
        pkey: &PublicKey,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        match response {
            Ok(Response::Found(s_addr)) => {
                info!("peer {} is at {}", pkey, s_addr);

                self.peer_cache.insert(*pkey, s_addr);

                self.connector
                    .establish(pkey, &s_addr)
                    .instrument(trace_span!("peer_connect"))
                    .await
            }
            Ok(Response::NotFound(pkey)) => {
                log_error!("directory server does not know peer {}", pkey);
                self.peer_cache.remove(&pkey);
                Err(IoError::from(ErrorKind::AddrNotAvailable).into())
            }
            Ok(_) => {
                log_error!("invalid response from directory server");
                Err(IoError::from(ErrorKind::AddrNotAvailable).into())
            }
            Err(e) => {
                log_error!("error reading response from directory: {}", e);
                Err(IoError::from(ErrorKind::BrokenPipe).into())
            }
        }
    }
}

#[async_trait]
impl Connector for DirectoryConnector {
    type Candidate = SocketAddr; // FIXME: make this a tuple of directory pkey and socketaddr

    fn exchanger(&self) -> &Exchanger {
        self.connector.exchanger()
    }

    /// Open a `Socket` to a peer using its `PublicKey` to find its `SocketAddr`
    /// from some directory server.
    ///
    /// # Arguments
    /// * `pkey`: `PublicKey` of the peer we are trying to connect to
    /// * `dir_addr`: Address of the directory server to search in
    async fn establish(
        &mut self,
        pkey: &PublicKey,
        dir_addr: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        info!("finding peer address for public key {}", pkey);

        if let Some(peer_addr) = self.peer_cache.get(pkey) {
            info!("found address {} for {} in cache", peer_addr, pkey);
            let result = self.connector.establish(pkey, peer_addr).await;

            if result.is_ok() {
                return result; // cache entry is valid
            }
        }

        // cache was stale or did not exist, fetch again
        let req = Request::Fetch(*pkey);

        let connection = self.find_directory_server(pkey, dir_addr).await?;

        if let Err(e) = connection.send_plain(&req).await {
            log_error!("directory server is unavailable: {}", e);
            return Err(IoError::from(ErrorKind::AddrNotAvailable).into());
        }

        let resp = connection.receive_plain::<Response>().await;

        self.handle_response(resp, pkey).await
    }
}
