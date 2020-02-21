use std::collections::HashMap;
use std::fmt;
use std::future::Future;
use std::io::{Error as IoError, ErrorKind};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use super::super::common::directory::{Info, Request, Response};
use super::super::{Connection, ReceiveError, SendError, Socket};
use super::{ConnectError, Connector};
use crate as drop;
use crate::crypto::key::exchange::{Exchanger, PublicKey};
use crate::error::Error;

use async_trait::async_trait;

use ccl::nestedmap::NestedMap;

use futures::future::{select, Either, FutureExt};
use futures::stream::StreamExt;

use macros::error;

use tokio::sync::broadcast::{channel, Receiver, Sender};
use tokio::task;

use tracing::{debug, error as log_error, info, trace_span, warn};
use tracing_futures::Instrument;

error! {
    type: DirectoryError,
    description: "directory server failure",
    causes: (ConnectError, IoError, SendError, ReceiveError)
}

/// A `Connector` that makes use of a centralized directory in order
/// to discover peers by their `PublicKey`. This `Connector` uses `PublicKey`s
/// as `Candidate` and finds out the actual address from the directory server.
pub struct Directory {
    /// `Connector` that will be used to open `Connection`s to peers
    connector: Arc<dyn Connector<Candidate = SocketAddr>>,
    /// Channels for requests to handlers
    handlers: NestedMap<Info, (Sender<Response>, Sender<Request>)>,
}

impl Directory {
    /// Create a new `DirectoryConnector` that will use the given `Connector` to
    /// establish connections to both the directory server and then to peers.
    ///
    /// # Arguments
    /// * `connector` the `Connector` that will be used to establish all
    /// `Connection`s including the `Connection` to the directory server
    pub fn new<C: Connector<Candidate = SocketAddr> + 'static>(
        connector: C,
    ) -> Self {
        Self {
            connector: Arc::new(connector),
            handlers: NestedMap::new(),
        }
    }

    /// Use this `DirectoryConnector` as a barrier. This method will wait until
    /// the specified `DirectoryServer` knows the address of `nr_peer` peers
    /// before returning, ensuring that the system in a usable state before
    /// continuing.
    ///
    /// # Arguments
    /// * `nr_peer` The number of peers to wait before returning
    /// * `info` The information (public key and address) needed to contact the
    /// directory server
    pub async fn wait(
        &mut self,
        nr_peer: usize,
        info: &Info,
    ) -> Result<Vec<Info>, DirectoryError> {
        let (mut rx, tx) = self.find_directory_handler(info).await?;
        let mut peers = Vec::with_capacity(nr_peer);
        let req = Request::Wait(nr_peer);
        let mut i = 0;

        tx.send(req).map_err(|_| {
            log_error!("failed to send message, handler died");
            IoError::new(ErrorKind::NotConnected, "")
        })?;

        debug!("waiting for {} peers in the directory", nr_peer);

        loop {
            if let Ok(peer) = rx.recv().await {
                if let Response::Found(pkey, addr) = peer {
                    info!("found peer {} at {}", pkey, addr);
                    peers.push((pkey, addr).into());
                }
            } else {
                log_error!("handler died, while waiting for directory");
            }

            i += 1;

            if i == nr_peer {
                break;
            }
        }

        info!("got {} peers from directory", nr_peer);
        Ok(peers)
    }

    async fn find_directory_handler(
        &self,
        info: &Info,
    ) -> Result<(Receiver<Response>, Sender<Request>), ConnectError> {
        let dir_addr = info.addr();
        let pkey = info.public();

        // `Entry` API does not support async so really no way to avoid double
        // lookup...
        if self.handlers.get(info).is_none() {
            let connection = self
                .connector
                .connect(pkey, &dir_addr)
                .instrument(trace_span!("directory_connect"))
                .await?;
            let (resp_tx, _) = channel(32);
            let (req_tx, req_rx) = channel(32);
            let handler = Handler::spawn(req_rx, resp_tx.clone(), connection);

            task::spawn(handler);

            self.handlers.insert(*info, (resp_tx, req_tx));
        }

        self.handlers
            .get(info)
            .map(|x| (x.0.subscribe(), x.1.clone()))
            .ok_or_else(|| {
                IoError::new(
                    ErrorKind::NotConnected,
                    "not connected to directory",
                )
                .into()
            })
    }
}

#[async_trait]
impl Connector for Directory {
    type Candidate = Info;

    fn exchanger(&self) -> &Exchanger {
        self.connector.exchanger()
    }

    /// Open a `Socket` to a peer using its `PublicKey` to find its `SocketAddr`
    /// from some directory server.
    ///
    /// # Arguments
    /// * `pkey` `PublicKey` of the peer we are trying to connect to
    /// * `dir_addr` Address of the directory server to search in
    async fn establish(
        &self,
        pkey: &PublicKey,
        directory_info: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        info!("finding peer address for public key {}", pkey);

        let (mut rx, tx) = self.find_directory_handler(directory_info).await?;

        if tx.send(Request::Fetch(*pkey)).is_err() {
            return Err(IoError::new(
                ErrorKind::NotConnected,
                "couldn't send request",
            )
            .into());
        }

        while let Ok(response) = rx.recv().await {
            match response {
                Response::Found(recvd_pkey, addr) if recvd_pkey == *pkey => {
                    return self.connector.establish(&pkey, &addr).await;
                }
                Response::Found(_, _) => continue,
                Response::NotFound(pkey) => {
                    log_error!("directory does not known {}", pkey);
                    return Err(IoError::new(
                        ErrorKind::NotConnected,
                        "unknown peer",
                    )
                    .into());
                }
                _ => {
                    return Err(IoError::new(
                        ErrorKind::InvalidData,
                        "protocol violation",
                    )
                    .into());
                }
            }
        }
        Err(IoError::new(ErrorKind::UnexpectedEof, "").into())
    }
}

/// This is an agent that takes care of sending requests to one directory server
/// and updating the local peer cache accordingly
struct Handler;

impl Handler {
    fn spawn(
        mut receiver: Receiver<Request>,
        mut notifier: Sender<Response>,
        mut connection: Connection,
    ) -> impl Future<Output = Result<(), DirectoryError>> {
        let peer_addr = connection
            .peer_addr()
            .map_or_else(|_| IpAddr::V4(Ipv4Addr::UNSPECIFIED), |x| x.ip());

        async move {
            let mut cache = HashMap::new();
            let mut request_opt = None;

            loop {
                let response_fut =
                    connection.receive_plain::<Response>().boxed();
                let request_fut = receiver.next().boxed();

                {
                    match select(response_fut, request_fut).await {
                        Either::Left((response, _)) => {
                            process_response(
                                response,
                                &mut cache,
                                &mut notifier,
                            )
                            .await?;
                        }
                        Either::Right((result, _)) => {
                            if let Some(Ok(request)) = result {
                                match request {
                                    Request::Fetch(pkey) => {
                                        request_opt = Some(request);

                                        if let Some(peer) = cache.get(&pkey) {
                                            if notifier.send(Response::Found(
                                                pkey, *peer,
                                            )).is_err() {
                                                log_error!("connector died, exiting handler");
                                                return Ok(());
                                            };
                                        }
                                    }
                                    _ => request_opt = Some(request),
                                }
                            } else {
                                info!("exiting handler");
                                return Ok(());
                            }
                        }
                    }
                }

                if let Some(ref request) = request_opt.take() {
                    connection.send_plain(request).await?;
                }
            }
        }
        .instrument(trace_span!("directory_handler", server=%peer_addr))
    }
}

async fn process_response(
    response: Result<Response, ReceiveError>,
    cache: &mut HashMap<PublicKey, SocketAddr>,
    notifier: &mut Sender<Response>,
) -> Result<(), ReceiveError> {
    match response {
        Err(e) => {
            log_error!("bad response received: {}", e);
            Err(e)
        }
        Ok(response) => {
            match response {
                Response::Found(pkey, addr) => {
                    cache.insert(pkey, addr);
                }
                Response::NotFound(pkey) => warn!("peer {} not found", pkey),
                Response::Ok => info!("end of directory listing"),
            }
            if notifier.send(response).is_err() {
                log_error!("no one waiting for response");
                Err(IoError::new(ErrorKind::NotConnected, "").into())
            } else {
                Ok(())
            }
        }
    }
}

#[derive(Clone, Copy, Eq, Hash, PartialEq)]
struct PeerInfo(IpAddr, u16);

impl fmt::Display for PeerInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.0, self.0)
    }
}

impl Into<SocketAddr> for PeerInfo {
    fn into(self) -> SocketAddr {
        (self.0, self.1).into()
    }
}

impl From<&PeerInfo> for SocketAddr {
    fn from(val: &PeerInfo) -> Self {
        (val.0, val.1).into()
    }
}

impl From<SocketAddr> for PeerInfo {
    fn from(addr: SocketAddr) -> Self {
        Self(addr.ip(), addr.port())
    }
}

impl From<&SocketAddr> for PeerInfo {
    fn from(addr: &SocketAddr) -> Self {
        Self::from(*addr)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::crypto::key::exchange::Exchanger;
    use crate::net::{DirectoryConnector, Listener, TcpConnector, TcpListener};
    use crate::test::*;

    #[tokio::test]
    async fn wait_whitebox() {
        init_logger();

        const NR_PEER: usize = 10;
        let connector = TcpConnector::new(Exchanger::random());
        let mut directory = Directory::new(connector);
        let server = next_test_ip4();
        let exchanger = Exchanger::random();
        let directory_exchanger = exchanger.clone();
        let peers: Vec<_> = (0..NR_PEER)
            .map(|_| (next_test_ip4(), Exchanger::random()))
            .collect();
        let peers_copy = peers.clone();
        let mut listener = TcpListener::new(server, exchanger)
            .await
            .expect("bind failed");

        let handle = task::spawn(async move {
            let peers = peers_copy;

            let mut connection =
                listener.accept().await.expect("accept failed");

            assert_eq!(
                connection
                    .receive_plain::<Request>()
                    .await
                    .expect("recv failed"),
                Request::Wait(NR_PEER),
                "bad message received from peer"
            );

            for (addr, exchanger) in peers {
                connection
                    .send_plain(&Response::Found(
                        *exchanger.keypair().public(),
                        addr,
                    ))
                    .await
                    .expect("send failed");
            }
        });

        let info = (*directory_exchanger.keypair().public(), server).into();
        let recv_peers =
            directory.wait(NR_PEER, &info).await.expect("wait failed");

        let keys: Vec<_> = recv_peers.iter().map(|x| *x.public()).collect();
        let addresses: Vec<_> = recv_peers.iter().map(|x| x.addr()).collect();

        assert_eq!(
            addresses,
            peers.iter().map(|x| x.0).collect::<Vec<_>>(),
            "address of peers are wrong"
        );
        assert_eq!(
            keys,
            peers
                .iter()
                .map(|x| *x.1.keypair().public())
                .collect::<Vec<_>>()
        );

        handle.await.expect("listener failed");
    }

    #[tokio::test]
    async fn establish_whitebox() {
        let server = next_test_ip4();
        let server_exchanger = Exchanger::random();
        let server_public = *server_exchanger.keypair().public();
        let directory_server = next_test_ip4();
        let directory_exchanger = Exchanger::random();
        let connector =
            DirectoryConnector::new(TcpConnector::new(Exchanger::random()));
        let mut listener = TcpListener::new(server, server_exchanger.clone())
            .await
            .expect("listen failed");
        let mut dir_listener =
            TcpListener::new(directory_server, directory_exchanger.clone())
                .await
                .expect("dir listen failed");
        let dir_info =
            (*directory_exchanger.keypair().public(), directory_server).into();

        let handle = task::spawn(async move {
            let mut connection =
                listener.accept().await.expect("accept failed");

            let msg = connection.receive::<u32>().await.expect("recv failed");
            assert_eq!(msg, 0u32, "wrong message received");
        });

        let dir_handle = task::spawn(async move {
            let mut connection =
                dir_listener.accept().await.expect("dir accept failed");

            let msg = connection
                .receive_plain::<Request>()
                .await
                .expect("dir recv failed");

            assert_eq!(msg, Request::Fetch(server_public));

            connection
                .send_plain(&Response::Found(server_public, server))
                .await
                .expect("dir send failed");
        });

        let mut connection = connector
            .connect(server_exchanger.keypair().public(), &dir_info)
            .await
            .expect("connect failed");

        connection.send(&0u32).await.expect("send failed");

        handle.await.expect("listener failed");
        dir_handle.await.expect("dir listener failed");
    }
}
