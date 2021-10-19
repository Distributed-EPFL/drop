use std::{collections::HashMap, fmt, future::Future, net::Ipv4Addr};

use futures::stream::{select_all, FuturesUnordered, Stream, StreamExt};
use tokio::{
    sync::mpsc,
    task::{self, JoinHandle},
};
use tokio_stream::wrappers::ReceiverStream;
use tracing::{debug_span, error, info, warn};
use tracing_futures::Instrument;

use crate::{
    crypto::key::exchange::PublicKey,
    net::{ConnectError, Connection, Connector, Listener, ListenerError},
};

/// System manager and related traits
mod manager;
pub use manager::*;

/// Wrappers around collections of `Connection` for easier use
mod sender;
pub use sender::*;

/// Sampling utilities
mod sampler;
pub use sampler::*;

/// Easy import path to use the system functionnality from drop
pub mod prelude {
    pub use super::{manager::*, sampler::*, sender::*};
}

/// A representation of a distributed `System` that manages connections to and
/// from other peers.
pub struct System {
    connections: HashMap<PublicKey, Connection>,
    listeners: Vec<JoinHandle<Result<(), ListenerError>>>,
    _listener_handles: Vec<JoinHandle<Result<(), ListenerError>>>,
    peer_input: Vec<mpsc::Receiver<Connection>>,
}

impl System {
    /// Create a new `System` using an `Iterator` over pairs of `PublicKey`s and
    /// `Connection` `Future`s
    pub async fn new<
        I: IntoIterator<Item = (PublicKey, F)>,
        F: Future<Output = Result<Connection, ConnectError>>,
    >(
        initial: I,
    ) -> Self {
        let iter = initial.into_iter();

        let connections = iter
            .map(|x| async {
                (
                    x.1.instrument(debug_span!("system_connect", dest = %x.0))
                        .await,
                    x.0,
                )
            })
            .collect::<FuturesUnordered<_>>()
            .filter_map(|(result, pkey)| async move {
                match result {
                    Ok(connection) => {
                        info!("connected to {}", pkey);
                        Some((pkey, connection))
                    }
                    Err(e) => {
                        error!("failed to connect to {}: {}", pkey, e);
                        None
                    }
                }
            })
            .collect::<HashMap<_, _>>()
            .await;

        Self {
            connections,
            ..Default::default()
        }
    }

    /// Create a new `System` using a list of peers and some `Connector`
    pub async fn new_with_connector_zipped<
        C: Connector<Candidate = CD>,
        CD: fmt::Display + Send + Sync,
        I: IntoIterator<Item = (PublicKey, CD)>,
    >(
        connector: &C,
        peers: I,
    ) -> Self {
        Self::new(peers.into_iter().map(|(pkey, candidate)| {
            (
                pkey,
                async move { connector.connect(&pkey, &candidate).await },
            )
        }))
        .await
    }

    /// Create a new `System` from an iterator of `Candidate`s and another of
    /// `PublicKey`s
    pub async fn new_with_connector<
        C: Connector<Candidate = CD>,
        CD: fmt::Display + Send + Sync,
        I1: IntoIterator<Item = PublicKey>,
        I2: IntoIterator<Item = CD>,
    >(
        connector: &C,
        pkeys: I1,
        candidates: I2,
    ) -> Self {
        Self::new_with_connector_zipped(
            connector,
            pkeys.into_iter().zip(candidates),
        )
        .await
    }

    /// Add a new peer into the `System` using the provided `Candidate` and
    /// `Connector`
    pub async fn add_peer<CD, C>(
        &mut self,
        connector: &C,
        candidates: &[CD],
        public: &PublicKey,
    ) -> Result<(), ConnectError>
    where
        CD: fmt::Display + Send + Sync,
        C: Connector<Candidate = CD>,
    {
        let connection = connector.connect_any(public, candidates).await?;

        self.connections.insert(*public, connection);

        Ok(())
    }

    /// Add many peers to this `System` using the provided `Connector`
    pub async fn add_peers<CD, C>(
        &mut self,
        connector: &C,
        candidates: &[(CD, PublicKey)],
    ) -> impl Iterator<Item = (PublicKey, ConnectError)>
    where
        CD: fmt::Display + Send + Sync,
        C: Connector<Candidate = CD>,
    {
        let (ok, err): (Vec<_>, Vec<_>) = connector
            .connect_many(candidates)
            .await
            .into_iter()
            .zip(candidates.iter().map(|x| x.1))
            .map(|(result, pkey)| match result {
                Ok(connection) => {
                    info!("connected to {}", pkey);
                    Ok((pkey, connection))
                }
                Err(e) => {
                    error!("failed to connect to {}: {}", pkey, e);
                    Err((pkey, e))
                }
            })
            .partition(Result::is_ok);

        self.connections.extend(ok.into_iter().map(Result::unwrap));

        err.into_iter().map(Result::unwrap_err)
    }

    /// Add a `Listener` to this `System` that will accept incoming peer
    /// `Connection`s
    pub async fn add_listener<C, L>(
        &mut self,
        mut listener: L,
    ) -> impl Stream<Item = ListenerError>
    where
        C: fmt::Display + Sync + Send,
        L: Listener<Candidate = C> + 'static,
    {
        let (err_tx, err_rx) = mpsc::channel(1);
        let (peer_tx, peer_rx) = mpsc::channel(32);

        let handle =
            task::spawn(async move {
                loop {
                    match listener.accept().await {
                        Err(e) => {
                            if let Err(e) = err_tx.send(e).await {
                                warn!(
                                    "lost error from listener on {}: {}",
                                    listener.local_addr().unwrap_or_else(
                                        || (Ipv4Addr::UNSPECIFIED, 0).into()
                                    ),
                                    e,
                                );
                            }
                        }
                        Ok(connection) => {
                            let _ = peer_tx.send(connection).await;
                        }
                    }
                }
            });

        self.peer_input.push(peer_rx);
        self.listeners.push(handle);

        ReceiverStream::new(err_rx)
    }

    /// Get all the `Connection`s known to this `System`.
    /// The returned `Connection`s will be removed from the system.
    pub fn connections(&mut self) -> Vec<Connection> {
        self.connections.drain().map(|x| x.1).collect()
    }

    /// Get a `Stream` that produces incoming `Connection`s from all registered
    /// `Listener`s. Subsequent calls to this method will only produces peers
    /// from `Listener`s that have been added *after* the previous call.
    pub fn peer_source(&mut self) -> impl Stream<Item = Connection> {
        select_all(self.peer_input.drain(..).map(ReceiverStream::new))
    }
}

impl Default for System {
    fn default() -> Self {
        Self {
            connections: Default::default(),
            listeners: Default::default(),
            _listener_handles: Vec::new(),
            peer_input: Vec::new(),
        }
    }
}

impl From<Vec<Connection>> for System {
    fn from(connections: Vec<Connection>) -> Self {
        Self {
            connections: connections
                .into_iter()
                .map(|x| (x.remote_key().unwrap(), x))
                .collect(),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use super::*;
    use crate::{
        crypto::key::exchange::Exchanger,
        net::{TcpConnector, TcpListener},
        test::*,
    };

    #[tokio::test]
    async fn add_peers() {
        init_logger();
        let addrs = test_addrs(11);
        let candidates = addrs
            .clone()
            .into_iter()
            .map(|(exchanger, addr)| (addr, *exchanger.keypair().public()))
            .collect::<Vec<_>>();
        let receivers =
            create_receivers(addrs.into_iter(), |mut connection| async move {
                let data = connection
                    .receive::<usize>()
                    .await
                    .expect("receive failed");

                assert_eq!(data, 0, "wrong data received");
            })
            .await;
        let mut system: System = Default::default();
        let connector = TcpConnector::new(Exchanger::random());
        let errors = system.add_peers(&connector, &candidates).await;

        let mut connections = system.connections();

        assert_eq!(errors.count(), 0, "error connecting to peers");

        connections
            .iter_mut()
            .map(|x| x.send(&0usize))
            .collect::<FuturesUnordered<_>>()
            .for_each(|result| async { result.expect("send failed") })
            .await;

        let connections = receivers
            .into_iter()
            .map(|(_, handle)| handle)
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;

        assert_eq!(connections.len(), 11, "not all connections opened");
    }

    #[tokio::test]
    async fn add_listener() {
        let mut system = System::default();
        let (exchanger, addr) = test_addrs(1).pop().unwrap();
        let pkey = *exchanger.keypair().public();

        let _ = system
            .add_listener(
                TcpListener::new(addr, exchanger)
                    .await
                    .expect("listen failed"),
            )
            .await;

        let exchanger = Exchanger::random();
        let client_pkey = *exchanger.keypair().public();
        let connector = TcpConnector::new(exchanger);

        connector
            .connect(&pkey, &addr)
            .await
            .expect("connect failed");

        assert_eq!(system.peer_input.len(), 1, "listener not added to system");

        let peer = system
            .peer_source()
            .next()
            .await
            .expect("unexpected end of stream");

        assert_eq!(
            peer.remote_key().unwrap(),
            client_pkey,
            "different addresses"
        );
    }
}
