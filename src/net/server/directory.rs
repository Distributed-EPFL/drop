use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use super::super::common::directory::*;
use super::super::listener::{Listener, ListenerError};
use super::super::Connection;
use super::ServerError;
use crate::crypto::key::exchange::PublicKey;

use futures::future::{self, Either};

use tokio::sync::broadcast::{channel as bcast_channel, Sender as BcastSender};
use tokio::sync::oneshot::{channel, Receiver, Sender};
use tokio::sync::RwLock;
use tokio::task;

use tracing::{debug, error, info, trace, trace_span, warn};
use tracing_futures::Instrument;

type PeerDirectory = Arc<RwLock<HashMap<PublicKey, SocketAddr>>>;

/// A server that serves directory requests from peers. The incoming
/// connection must be plain text to avoid having to know a public key for
/// the directory server.
/// # Example
/// ```
/// # use std::net::{Ipv4Addr, SocketAddr};
/// use drop::net::{DirectoryServer, TcpListener, ServerError};
/// use drop::crypto::key::exchange::Exchanger;
///
/// # async fn doc() -> Result<(), ServerError> {
/// let addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, 0).into();
/// let tcp = TcpListener::new(addr, Exchanger::random()).await
///     .expect("bind failed");
/// let (server, exit) = DirectoryServer::new(tcp);
/// let handle = tokio::task::spawn(async move {
///      server.serve().await
/// });
/// exit.send(());
/// handle.await.expect("server failure");
/// # Ok(())
/// # }
/// ```
pub struct DirectoryServer {
    peers: PeerDirectory,
    listener: Box<dyn Listener<Candidate = SocketAddr>>,
    exit: Receiver<()>,
    sender: BcastSender<Info>,
}

impl DirectoryServer {
    /// Create a new directory server that will use the provided `Listener`
    /// to accept incoming directory `Connection`s.
    pub fn new<L: Listener<Candidate = SocketAddr> + 'static>(
        listener: L,
    ) -> (Self, Sender<()>) {
        let (tx, rx) = channel();
        let (sender, _) = bcast_channel(32);

        (
            Self {
                listener: Box::new(listener),
                peers: PeerDirectory::default(),
                exit: rx,
                sender,
            },
            tx,
        )
    }

    /// Serve requests according to parameters given at server creation
    pub async fn serve(mut self) -> Result<(), ListenerError> {
        let mut exit_fut = Some(self.exit);

        loop {
            let (exit, connection) = match Self::poll_incoming(
                self.listener.as_mut(),
                exit_fut.take().unwrap(),
            )
            .await
            {
                PollResult::Error(e) => {
                    error!("failed to accept incoming connection: {}", e);
                    return Err(e);
                }
                PollResult::Exit => {
                    info!("directory server exiting...");
                    return Ok(());
                }
                PollResult::Incoming(exit, connection) => (exit, connection),
            };

            exit_fut = Some(exit);

            let peer_addr = connection.peer_addr()?;

            info!("new directory connection from {}", peer_addr);

            let peers = self.peers.clone();
            let tx = self.sender.clone();

            task::spawn(
                async move {
                    let servicer = PeerServicer::new(connection, peers, tx);

                    if let Err(e) = servicer.serve().await {
                        error!("failed to service peer: {}", e);
                    }
                }
                .instrument(trace_span!("peer_service", client = %peer_addr)),
            );

            info!("waiting for next connection");
        }
    }

    async fn poll_incoming<L: Listener<Candidate = SocketAddr> + ?Sized>(
        listener: &mut L,
        exit: Receiver<()>,
    ) -> PollResult {
        match future::select(exit, listener.accept()).await {
            Either::Left(_) => PollResult::Exit,
            Either::Right((Ok(connection), exit)) => {
                PollResult::Incoming(exit, connection)
            }
            Either::Right((Err(e), _)) => PollResult::Error(e),
        }
    }
}

enum PollResult {
    Incoming(Receiver<()>, Connection),
    Error(ListenerError),
    Exit,
}

struct PeerServicer {
    peers: PeerDirectory,
    connection: Connection,
    /// Broadcast channel to let other `PeerService` know a peer was added
    sender: BcastSender<Info>,
}

impl PeerServicer {
    fn new(
        connection: Connection,
        peers: PeerDirectory,
        sender: BcastSender<Info>,
    ) -> Self {
        Self {
            peers,
            connection,
            sender,
        }
    }

    /// Notify other `PeerServicer` that a new peer has been added
    async fn notify(&mut self, peer: &Info) -> Result<(), ()> {
        // errors just mean no outstanding wait request
        self.sender.send(*peer).map(|_| ()).map_err(|_| ())
    }

    async fn send_peer(&mut self, peer: &Info) -> Result<(), ServerError> {
        Ok(self
            .connection
            .send_plain(&Response::Found(*peer.public(), peer.addr()))
            .await?)
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

        let info = (*peer.public(), peer.addr()).into();

        if self.notify(&info).await.is_err() {
            warn!("no peer is waiting on directory listing");
        } else {
            debug!("notified of peer {}", info);
        }

        Response::Ok
    }

    async fn handle_wait(
        &mut self,
        peer_nr: usize,
    ) -> Result<Response, ServerError> {
        info!("wait request for {} peers", peer_nr);

        let sent = self.flush_existing(peer_nr).await?;

        debug!("sent {} peers out of {}", sent, peer_nr);

        if sent < peer_nr {
            self.wait_notify(sent, peer_nr).await?;
        }

        info!("wait request complete");

        Ok(Response::Ok)
    }

    async fn wait_notify(
        &mut self,
        sent: usize,
        expected: usize,
    ) -> Result<(), ServerError> {
        let mut receiver = self.sender.subscribe();

        for _ in sent..expected {
            if let Ok(peer) = receiver.recv().await {
                debug!("new peer {}", peer);
                self.send_peer(&peer).await?;
            } else {
                warn!("server has stopped, we won't get any new peers");
                return Ok(());
            }
        }
        Ok(())
    }

    async fn flush_existing(
        &mut self,
        max: usize,
    ) -> Result<usize, ServerError> {
        let mut count = 0;

        for (pkey, addr) in self.peers.read().await.iter() {
            if count >= max {
                break;
            }
            count += 1;
            self.connection
                .send_plain(&Response::Found(*pkey, *addr))
                .await?;
        }

        Ok(count)
    }

    /// Serve directory request to the peer we are connected to.
    async fn serve(mut self) -> Result<(), ServerError> {
        info!("servicing directory request");

        while let Ok(request) = self.connection.receive_plain::<Request>().await
        {
            trace!("received {:?}", request);
            let response = match request {
                Request::Fetch(ref pkey) => self.handle_fetch(pkey).await,
                Request::Add(ref peer) => self.handle_add(peer).await,
                Request::Wait(peer_nr) => self.handle_wait(peer_nr).await?,
            };

            trace!("sending response {:?}", response);

            self.connection.send_plain(&response).await?;
        }

        self.connection.close().await?;

        info!("end of directory connection");

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::super::super::connector::Connector;
    use super::*;
    use crate::crypto::key::exchange::Exchanger;
    use crate::net::{TcpConnector, TcpListener};
    use crate::test::*;

    use futures::future;

    use tokio::task::{self, JoinHandle};

    async fn setup_server(server: SocketAddr) -> (Sender<()>, JoinHandle<()>) {
        let server_exchanger = Exchanger::random();
        let listener = TcpListener::new(server, server_exchanger)
            .await
            .expect("listen failed");
        let (dir_server, exit_tx) = DirectoryServer::new(listener);

        let handle = task::spawn(
            async move { dir_server.serve().await.expect("serve failed") }
                .instrument(trace_span!("directory_serve")),
        );

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
        connector: &dyn Connector<Candidate = SocketAddr>,
    ) -> Connection {
        let peer = (pkey, addr).into();
        let req = Request::Add(peer);

        let mut connection = connector
            .connect(&pkey, &server)
            .instrument(trace_span!("adder"))
            .await
            .expect("connect failed");
        let local = connection.local_addr().expect("getaddr failed");

        async move {
            connection.send_plain(&req).await.expect("send failed");

            let resp = connection
                .receive_plain::<Response>()
                .await
                .expect("recv failed");

            assert_eq!(resp, Response::Ok, "invalid response");

            connection
        }
        .instrument(trace_span!("adder", client = %local))
        .await
    }

    async fn wait_for_server(exit_tx: Sender<()>, handle: JoinHandle<()>) {
        exit_tx.send(()).expect("exit_failed");
        handle.await.expect("server failed");
    }

    #[tokio::test(threaded_scheduler)]
    async fn serve_many() {
        init_logger();
        const TOTAL: usize = 10;
        let server = next_test_ip4();
        let (exit_tx, handle) = setup_server(server).await;
        let mut handles = Vec::new();

        for i in 1..TOTAL {
            handles.push(task::spawn(async move {
                let count = i * 2;
                let req = Request::Wait(count);
                let mut peers = Vec::new();
                let connector = TcpConnector::new(Exchanger::random());
                let (pkey, addr) = new_peer();
                let mut connection =
                    add_peer(server, addr, pkey, &connector).await;

                connection.send_plain(&req).await.expect("send failed");

                for _ in 0..count {
                    let peer = connection
                        .receive_plain::<Response>()
                        .await
                        .expect("recv failed");

                    peers.push(peer);
                }

                let end = connection
                    .receive_plain::<Response>()
                    .await
                    .expect("recv failed");

                assert_eq!(end, Response::Ok, "invalid end of list");
                assert_eq!(count, peers.len(), "incorrect number of peers");
            }));

            handles.push(task::spawn(async move {
                let connector = TcpConnector::new(Exchanger::random());
                let (pkey, peer_addr) = new_peer();
                add_peer(server, peer_addr, pkey, &connector).await;
            }));
        }

        future::join_all(handles)
            .await
            .drain(..)
            .collect::<Result<Vec<_>, _>>()
            .expect("some task failed");

        wait_for_server(exit_tx, handle).await;
    }

    #[tokio::test(threaded_scheduler)]
    async fn wait_for_enough_peers() {
        init_logger();
        const TOTAL: usize = 10;
        let server = next_test_ip4();
        let (exit_tx, handle) = setup_server(server).await;
        let connector = TcpConnector::new(Exchanger::random());
        let (pkey, addr) = new_peer();

        let mut connection = add_peer(server, addr, pkey, &connector).await;
        let (tx, rx) = channel();

        task::spawn(async move {
            connection
                .send_plain(&Request::Wait(TOTAL))
                .await
                .expect("wait failed");

            let mut count = 0;
            tx.send(()).expect("notify failed");

            while let Ok(_) = connection.receive_plain::<Response>().await {
                count += 1;
            }

            let end = connection
                .receive_plain::<Response>()
                .await
                .expect("recv failed");

            assert_eq!(end, Response::Ok, "wrong delimiter");
            assert_eq!(count, TOTAL, "wrong number of peers");
        });

        // delay until wait request has been sent to trigger the active wait case
        rx.await.expect("recv failed");

        let futures = (0..TOTAL).map(|_| {
            task::spawn(async move {
                let connector = TcpConnector::new(Exchanger::random());
                let (pkey, addr) = new_peer();
                add_peer(server, addr, pkey, &connector).await;
            })
        });

        future::join_all(futures)
            .await
            .drain(..)
            .collect::<Result<Vec<_>, _>>()
            .expect("some task failed");

        wait_for_server(exit_tx, handle).await;
    }

    #[tokio::test]
    async fn single_wait() {
        init_logger();
        let server = next_test_ip4();
        let (exit_tx, handle) = setup_server(server).await;
        let connector = TcpConnector::new(Exchanger::random());
        const TOTAL: usize = 3;

        let (pkey, peer) = new_peer();
        let mut w_connection = add_peer(server, peer, pkey, &connector).await;

        let waiter = task::spawn(
            async move {
                w_connection
                    .send_plain(&Request::Wait(3))
                    .await
                    .expect("wait failed");

                for _ in 0..TOTAL {
                    w_connection
                        .receive_plain::<Response>()
                        .await
                        .expect("recv failed");
                }
            }
            .instrument(trace_span!("waiter")),
        );

        for _ in 0..TOTAL {
            let (pkey, peer) = new_peer();

            add_peer(server, peer, pkey, &connector).await;
        }

        wait_for_server(exit_tx, handle).await;

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
                {
                    task::spawn(async move {
                        let exc = Exchanger::random();
                        let public = *exc.keypair().public();
                        let connector = TcpConnector::new(exc);

                        let mut connection = connector
                            .connect(&public, &server)
                            .await
                            .expect("connect failed");

                        connection
                            .send_plain(&Request::Wait(TOTAL))
                            .await
                            .expect("send failed");

                        let mut peers = Vec::new();

                        for _ in 0..TOTAL {
                            let peer = connection
                                .receive_plain::<Response>()
                                .await
                                .expect("recv failed");
                            peers.push(peer);
                        }

                        assert_eq!(TOTAL, peers.len(), "wrong number of peers");
                    })
                }
                .instrument(trace_span!("waiter"))
            })
            .collect::<Vec<_>>();

        let exch = Exchanger::random();
        let connector = TcpConnector::new(exch);

        for _ in 0..TOTAL {
            let (pkey, addr) = new_peer();

            add_peer(server, addr, pkey, &connector)
                .await
                .send_plain(&0u32)
                .await
                .expect("failed to close connection");
        }

        future::join_all(handles)
            .await
            .drain(..)
            .collect::<Result<Vec<_>, _>>()
            .expect("failed to join peers");

        wait_for_server(exit_tx, handle).await;
    }

    #[tokio::test]
    async fn add_then_fetch() {
        let server = next_test_ip4();
        let (exit_tx, handle) = setup_server(server).await;
        let connector = TcpConnector::new(Exchanger::random());

        let peer_addr = next_test_ip4();
        let peer_pkey = *Exchanger::random().keypair().public();
        let mut connection =
            add_peer(server, peer_addr, peer_pkey, &connector).await;

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

        let connector = TcpConnector::new(Exchanger::random());
        let public = *connector.exchanger().keypair().public();
        let mut connection = connector
            .connect(&public, &server)
            .await
            .expect("connect failed");

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
