use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::future::Future;
use std::hash::Hash;
use std::net::{
    IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6,
};
use std::sync::Arc;

use crate::crypto::key::exchange::PublicKey;
use crate::net::{
    ConnectError, Connection, Connector, Listener, ListenerError,
};

use futures::future;
use futures::stream::{select_all, Stream};

use serde::{Deserialize, Serialize};

use tokio::sync::mpsc;
use tokio::task::{self, JoinHandle};

use tracing::{debug_span, error, info, warn};
use tracing_futures::Instrument;

pub use drop_derive::message;

/// System manager and related traits
pub mod manager;
pub use manager::Processor;

/// Wrappers around collections of `Connection` for easier use
pub mod sender;
pub use sender::{Sender, SenderError};

/// Sampling utilities
pub mod sampler;
pub use sampler::{SampleError, Sampler};

#[cfg(test)]
pub mod test;

/// A trait bound for types that can be used as messages
pub trait Message:
    for<'de> Deserialize<'de>
    + Serialize
    + fmt::Debug
    + Send
    + Sync
    + Clone
    + Hash
    + PartialEq
    + Eq
{
}

macro_rules! impl_m {
    ( $($t:ty),* ) => {
        $( impl Message for $t {} )*
    };
}

impl_m!(
    char,
    bool,
    u8,
    i8,
    u16,
    i16,
    u32,
    i32,
    u64,
    i64,
    u128,
    i128,
    isize,
    usize,
    String,
    SocketAddr,
    SocketAddrV4,
    SocketAddrV6,
    IpAddr,
    Ipv4Addr,
    Ipv6Addr
);

macro_rules! impl_g {
    ( $($t:ty),* ) => {
        $(impl<T: Message> Message for $t {})*
    }
}

impl_g!(Vec<T>, VecDeque<T>, Box<T>, Arc<T>);

macro_rules! impl_a {
    ( $($sz:expr),* ) => {
        $( impl<T: Message> Message for [T; $sz] {} )*
    };
}

impl_a!(
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
    21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32
);

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

        let connections = future::join_all(iter.map(|x| async {
            (
                x.1.instrument(debug_span!("system_connect", dest = %x.0))
                    .await,
                x.0,
            )
        }))
        .await
        .drain(..)
        .filter_map(|(result, pkey)| match result {
            Ok(connection) => {
                info!("connected to {}", pkey);
                Some((pkey, connection))
            }
            Err(e) => {
                error!("failed to connect to {}: {}", pkey, e);
                None
            }
        })
        .map(|(pkey, connection)| (pkey, connection))
        .collect::<HashMap<_, _>>();

        let mut result = Self::default();

        result.connections = connections;

        result
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
    ) -> impl Iterator<Item = ConnectError>
    where
        CD: fmt::Display + Send + Sync,
        C: Connector<Candidate = CD>,
    {
        let (ok, err): (Vec<_>, Vec<_>) = connector
            .connect_many(candidates)
            .await
            .drain(..)
            .zip(candidates.iter().map(|x| x.1))
            .map(|(result, pkey)| match result {
                Ok(connection) => {
                    info!("connected to {}", pkey);
                    Ok((pkey, connection))
                }
                Err(e) => {
                    error!("failed to connect to {}: {}", pkey, e);
                    Err(e)
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
        let (mut err_tx, err_rx) = mpsc::channel(1);
        let (mut peer_tx, peer_rx) = mpsc::channel(32);

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

        err_rx
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
        select_all(self.peer_input.drain(..))
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
    use super::test::*;
    use super::*;

    use crate::crypto::key::exchange::Exchanger;
    use crate::net::{TcpConnector, TcpListener};

    use futures::StreamExt;

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

        future::join_all(connections.iter_mut().map(|x| async move {
            x.send(&0usize).await.expect("send failed");
        }))
        .await;

        future::join_all(receivers.into_iter().map(|(_, handle)| handle)).await;

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
