use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use super::super::common::directory::*;
use super::super::listener::{Listener, ListenerError};
use super::super::Connection;
use super::*;
use crate::crypto::key::exchange::PublicKey;

use snafu::ResultExt;

use tokio::sync::broadcast::{
    channel as bcast_channel, Receiver as BcastReceiver, Sender as BcastSender,
};
use tokio::sync::oneshot::{channel, Receiver, Sender};
use tokio::sync::RwLock;
use tokio::task;
use tokio::time::timeout;

use tracing::{debug, error, info, trace, trace_span, warn};
use tracing_futures::Instrument;

type PeerDirectory = Arc<RwLock<HashMap<PublicKey, SocketAddr>>>;

/// A server that serves directory requests from peers. The incoming
/// connection must be plain text to avoid having to know a public key for
/// the directory server.
pub struct DirectoryServer {
    peers: PeerDirectory,
    listener: Box<dyn Listener<Candidate = SocketAddr>>,
    exit: Receiver<()>,
    sender: BcastSender<usize>,
}

impl DirectoryServer {
    /// Create a new directory server that will use the provided `Listener`
    /// to accept incoming directory `Connection`s.
    pub fn new(
        listener: Box<dyn Listener<Candidate = SocketAddr>>,
    ) -> (Self, Sender<()>) {
        let (tx, rx) = channel();
        let (sender, _) = bcast_channel(32);

        (
            Self {
                listener,
                peers: PeerDirectory::default(),
                exit: rx,
                sender,
            },
            tx,
        )
    }

    /// Serve requests according to parameters given at server creation
    pub async fn serve(mut self) -> Result<(), ServerError> {
        let to = Duration::from_secs(1);
        loop {
            if self.exit.try_recv().is_ok() {
                info!("stopping directory server");
                break;
            }

            let socket = match timeout(to, self.listener.establish()).await {
                Ok(Ok(socket)) => socket,
                Ok(Err(e)) => {
                    error!("failed to accept directory connection: {}", e);
                    return Err(e.into());
                }
                Err(_) => continue,
            };

            let peer_addr = socket.peer_addr().context(ServerIo {
                when: "fetching peer address",
            })?;

            info!("new directory connection from {}", peer_addr);

            let peers = self.peers.clone();
            let (tx, rx) = (self.sender.clone(), self.sender.subscribe());

            task::spawn(
                async move {
                    let servicer = PeerServicer::new(
                        Connection::new(socket),
                        peers,
                        tx,
                        rx,
                    );

                    if let Err(e) = servicer.serve().await {
                        error!("failed to service peer: {}", e);
                    }
                }
                .instrument(trace_span!("peer_service", client = %peer_addr)),
            );

            info!("waiting for next connection");
        }

        Ok(())
    }
}

struct PeerServicer {
    peers: PeerDirectory,
    connection: Connection,
    /// Broadcast channel to let other `PeerService` know a peer was added
    sender: BcastSender<usize>,
    /// Broadcast receiver to receive notifications from other `PeerServicer`
    receiver: BcastReceiver<usize>,
}

impl PeerServicer {
    fn new(
        connection: Connection,
        peers: PeerDirectory,
        sender: BcastSender<usize>,
        receiver: BcastReceiver<usize>,
    ) -> Self {
        Self {
            peers,
            connection,
            sender,
            receiver,
        }
    }

    /// Notify other `PeerServicer` that a new peer has been added
    async fn notify(&mut self) -> Result<(), ()> {
        self.sender
            .send(self.peers.read().await.len())
            .map(|_| ())
            .map_err(|_| ())
    }

    /// List current content of the directory to the remote peer
    async fn list_directory(&mut self) -> Result<(), ServerError> {
        for peer in self.peers.read().await.iter() {
            let peer: Info = (*peer.0, *peer.1).into();
            self.connection.send_plain(&peer).await.context(Send {
                when: "listing directory",
            })?;
        }
        Ok(())
    }

    /// Fetch and address from the directory by its `PublicKey`
    async fn handle_fetch(&mut self, pkey: &PublicKey) -> Response {
        info!("request for {}", pkey);

        if let Some(addr) = self.peers.read().await.get(pkey) {
            Response::Found(*pkey, *addr)
        } else {
            Response::NotFound(*pkey)
        }
    }

    async fn handle_add(&mut self, peer: &Info) -> Response {
        info!("request to add {}", peer);

        self.peers.write().await.insert(*peer.public(), peer.addr());

        if self.notify().await.is_err() {
            error!("no peer is waiting on directory listing");
        }

        Response::Ok
    }

    async fn handle_wait(&mut self, peer_nr: usize) {
        debug!("peer wants to wait for {} total peers", peer_nr);

        if self.peers.read().await.len() < peer_nr {
            info!("not enough peers, waiting for more...");
            loop {
                if let Ok(count) = self.receiver.recv().await {
                    if count == peer_nr {
                        break;
                    }
                } else {
                    warn!("all other peer died, stopping wait");
                    task::yield_now().await;
                }
            }
        }
    }

    /// Serve directory request to the peer we are connected to.
    async fn serve(mut self) -> Result<(), ServerError> {
        info!("servicing directory request");

        while let Ok(request) = self.connection.receive_plain::<Request>().await
        {
            let response = match request {
                Request::Fetch(ref pkey) => self.handle_fetch(pkey).await,
                Request::Add(ref peer) => self.handle_add(peer).await,
                Request::Wait(peer_nr) => {
                    self.handle_wait(peer_nr).await;
                    info!(
                        "reached {} peers in the system, notifying...",
                        peer_nr
                    );

                    self.list_directory().await?;

                    Response::Ok
                }
            };

            trace!("sending response {:?}", response);

            self.connection.send_plain(&response).await.context(Send {
                when: "responding to request",
            })?;
        }

        error!("corrupted request");

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::super::super::connector::{Connector, TcpDirect};
    use super::super::super::listener::TcpListener;
    use super::*;
    use crate::crypto::key::exchange::Exchanger;
    use crate::test::*;

    use tokio::task::{self, JoinHandle};

    async fn setup_server(server: SocketAddr) -> (Sender<()>, JoinHandle<()>) {
        let server_exchanger = Exchanger::random();
        let listener = Box::new(
            TcpListener::new(server, server_exchanger)
                .await
                .expect("listen failed"),
        );
        let (dir_server, exit_tx) = DirectoryServer::new(listener);

        let handle = task::spawn(async move {
            dir_server.serve().await.expect("serve failed")
        });

        (exit_tx, handle)
    }

    fn new_peer() -> (PublicKey, SocketAddr) {
        let peer = next_test_ip4();
        let pkey = *Exchanger::random().keypair().public();

        (pkey, peer)
    }

    async fn add_peer(
        server: SocketAddr,
        addr: SocketAddr,
        pkey: PublicKey,
        connector: &mut dyn Connector<Candidate = SocketAddr>,
    ) -> Connection {
        let peer = (pkey, addr).into();
        let req = Request::Add(peer);

        let mut connection = Connection::new(
            connector
                .establish(&pkey, &server)
                .await
                .expect("connect failed"),
        );

        connection.send_plain(&req).await.expect("send failed");

        let resp = connection
            .receive_plain::<Response>()
            .await
            .expect("recv failed");

        assert_eq!(resp, Response::Ok, "invalid response");

        connection
    }

    async fn wait_for_server(exit_tx: Sender<()>, handle: JoinHandle<()>) {
        exit_tx.send(()).expect("exit_failed");
        handle.await.expect("server failed");
    }

    #[tokio::test]
    async fn serve_many() {
        init_logger();
        let server = next_test_ip4();
        let mut connector = TcpDirect::new(Exchanger::random());
        let (exit_tx, handle) = setup_server(server).await;

        for i in 1..10usize {
            let (pkey, peer_addr) = new_peer();
            let mut connection =
                add_peer(server, peer_addr, pkey, &mut connector).await;
            let req = Request::Wait(i);

            connection.send_plain(&req).await.expect("send failed");
            let mut peers = Vec::new();

            while let Ok(peer) = connection.receive_plain::<Info>().await {
                peers.push(peer);
            }

            assert_eq!(i, peers.len(), "incorrect number of peers");
        }

        wait_for_server(exit_tx, handle).await;
    }

    #[tokio::test]
    async fn single_wait() {
        init_logger();
        let server = next_test_ip4();
        let (exit_tx, handle) = setup_server(server).await;
        let mut connector = TcpDirect::new(Exchanger::random());

        let (pkey, peer) = new_peer();
        let mut w_connection =
            add_peer(server, peer, pkey, &mut connector).await;

        let waiter = task::spawn(async move {
            w_connection
                .send_plain(&Request::Wait(3))
                .await
                .expect("wait failed");

            let mut i = 0usize;

            while let Ok(_) = w_connection.receive_plain::<Info>().await {
                i += 1;
            }

            assert_eq!(i, 3, "waited for wrong number of peer");

            wait_for_server(exit_tx, handle).await;
        });

        for _ in 0..2usize {
            let peer = next_test_ip4();
            let pkey = *Exchanger::random().keypair().public();

            task::yield_now().await;
            add_peer(server, peer, pkey, &mut connector).await;
        }

        waiter.await.expect("waiter failed");
    }

    #[tokio::test]
    async fn multi_wait() {
        init_logger();
        let server = next_test_ip4();
        let (exit_tx, handle) = setup_server(server).await;
        const TOTAL: usize = 10;

        let handles = (0..TOTAL)
            .map(|_| {
                task::spawn(async move {
                    let exc = Exchanger::random();
                    let public = *exc.keypair().public();
                    let mut connector = TcpDirect::new(exc);

                    let mut connection = Connection::new(
                        connector
                            .establish(&public, &server)
                            .await
                            .expect("connect failed"),
                    );

                    connection
                        .send_plain(&Request::Wait(TOTAL))
                        .await
                        .expect("send failed");

                    let mut peers = Vec::new();
                    while let Ok(peer) =
                        connection.receive_plain::<Info>().await
                    {
                        peers.push(peer);
                    }

                    assert_eq!(TOTAL, peers.len(), "wrong number of peers");
                })
            })
            .collect::<Vec<_>>();

        let exch = Exchanger::random();
        let public = *exch.keypair().public();

        let mut connection = Connection::new(
            TcpDirect::new(exch)
                .establish(&public, &server)
                .await
                .expect("connect failed"),
        );

        for _ in 0..TOTAL {
            let dir_peer = new_peer().into();
            connection
                .send_plain(&Request::Add(dir_peer))
                .await
                .expect("add failed");

            let resp = connection
                .receive_plain::<Response>()
                .await
                .expect("add failed");

            assert_eq!(resp, Response::Ok, "bad response");
        }

        for handle in handles {
            handle.await.expect("client failure");
        }
        wait_for_server(exit_tx, handle).await;
    }

    #[tokio::test]
    async fn add_then_fetch() {
        let server = next_test_ip4();
        let (exit_tx, handle) = setup_server(server).await;
        let mut connector = TcpDirect::new(Exchanger::random());

        let peer_addr = next_test_ip4();
        let peer_pkey = *Exchanger::random().keypair().public();
        let mut connection =
            add_peer(server, peer_addr, peer_pkey, &mut connector).await;

        connection
            .send_plain(&Request::Fetch(peer_pkey))
            .await
            .expect("fetch failed");

        let resp = connection
            .receive_plain::<Response>()
            .await
            .expect("recv failed");

        assert_eq!(
            resp,
            Response::Found(peer_pkey, peer_addr),
            "wrong directory entry"
        );

        wait_for_server(exit_tx, handle).await;
    }

    #[tokio::test]
    async fn empty_fetch() {
        let server = next_test_ip4();
        let (exit_tx, handle) = setup_server(server).await;

        let mut connector = TcpDirect::new(Exchanger::random());
        let public = *connector.exchanger().keypair().public();
        let mut connection = Connection::new(
            connector
                .establish(&public, &server)
                .await
                .expect("connect failed"),
        );

        connection
            .send_plain(&Request::Fetch(public))
            .await
            .expect("send failed");

        let resp = connection
            .receive_plain::<Response>()
            .await
            .expect("recv failed");

        assert_eq!(resp, Response::NotFound(public), "wrong response");

        wait_for_server(exit_tx, handle).await;
    }
}
