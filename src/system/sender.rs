use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
use std::ops::Deref;
use std::sync::Arc;

use super::Message;
use crate::async_trait;
use crate::crypto::key::exchange::PublicKey;
use crate::net::{ConnectionWrite, SendError};

use futures::future;
use futures::stream::{FuturesUnordered, Stream, StreamExt, TryStreamExt};

use snafu::{ensure, OptionExt, ResultExt, Snafu};

use tokio::sync::{Mutex, RwLock};

use tracing::warn;

#[derive(Debug, Snafu)]
/// Error returned by `Sender` when attempting to send `Message`s
pub enum SenderError {
    #[snafu(display("peer {} is unknown", remote))]
    /// The destination `PublicKey` was not known by this `Sender`
    NoSuchPeer { remote: PublicKey },
    #[snafu(display("connection with {} is broken: {}", remote, source))]
    /// The `Connection` encountered an error while sending
    ConnectionError {
        remote: PublicKey,
        source: SendError,
    },
    #[snafu(display("{} send errors", errors.len()))]
    /// Many send errors were encountered
    ManyErrors { errors: Vec<SenderError> },
}

#[async_trait]
/// Trait used when sending messages out from `Processor`s.
pub trait Sender<M: Message + 'static>: Send + Sync {
    /// Add a new `ConnectionWrite` to this `Sender`
    async fn add_connection(&self, write: ConnectionWrite);

    /// Get the keys of  peers known by this `Sender`
    ///
    /// # Returns
    /// A `Vec` containing the keys of all peers to which there is an outbound `Connection`
    /// at this time.
    async fn keys(&self) -> Vec<PublicKey>;

    /// Send a message to a given peer using this `Sender`
    async fn send(
        &self,
        message: Arc<M>,
        pkey: &PublicKey,
    ) -> Result<(), SenderError>;

    /// Send a set of messages to a remote peer
    ///
    /// # Returns
    /// An `Err` if any message fails to be sent, `Ok` otherwise
    async fn send_many_to_one<'a, I>(
        &self,
        messages: I,
        to: &PublicKey,
    ) -> Result<(), SenderError>
    where
        I: IntoIterator<Item = Arc<M>> + Send,
        I::IntoIter: Send,
    {
        messages
            .into_iter()
            .map(|message| self.send(message, to))
            .collect::<FuturesUnordered<_>>()
            .try_fold((), |_, _| future::ready(Ok(())))
            .await
    }

    /// Send a set of messages contained in an async `Stream` to a remote peer
    ///
    /// # Returns
    /// An Err([`SenderError`]) if any single message fails to be sent, `Ok` otherwise
    ///
    /// [`SenderError`]: self::SenderError
    /// [`Stream`]: futures::stream::Stream
    async fn send_many_to_one_stream<'a, S>(
        &self,
        messages: S,
        to: &PublicKey,
    ) -> Result<(), SenderError>
    where
        S: Stream<Item = Arc<M>> + Send,
    {
        messages
            .then(|message| self.send(message, to))
            .try_fold((), |_, _| future::ready(Ok(())))
            .await
    }

    /// Send the same message to many different peers.
    ///
    /// # Returns
    ///
    /// An `Err` if any one message failed to be sent, `Ok` otherwise
    async fn send_many<'a, I: Iterator<Item = &'a PublicKey> + Send>(
        &self,
        message: Arc<M>,
        keys: I,
    ) -> Result<(), SenderError> {
        let errors = keys
            .map(|key| {
                let message = message.clone();
                self.send(message, &key)
            })
            .collect::<FuturesUnordered<_>>();

        let errors = errors
            .filter_map(|x| async move { x.err() })
            .collect::<Vec<_>>()
            .await;

        if errors.is_empty() {
            Ok(())
        } else {
            ManyErrors { errors }.fail()
        }
    }
}

/// A handle to send messages to other known processes
pub struct NetworkSender<M: Message> {
    connections: RwLock<HashMap<PublicKey, Mutex<ConnectionWrite>>>,
    _m: PhantomData<M>,
}

impl<M: Message> NetworkSender<M> {
    /// Create a new `Sender` from a `Vec` of `ConnectionWrite`
    pub fn new<I: IntoIterator<Item = ConnectionWrite>>(writes: I) -> Self {
        let connections = writes
            .into_iter()
            .map(|x| (*x.remote_pkey(), Mutex::new(x)))
            .collect::<HashMap<_, _>>();

        Self {
            connections: RwLock::new(connections),
            _m: PhantomData,
        }
    }
}

#[async_trait]
impl<M: Message + 'static> Sender<M> for NetworkSender<M> {
    async fn send(
        &self,
        message: Arc<M>,
        pkey: &PublicKey,
    ) -> Result<(), SenderError> {
        self.connections
            .read()
            .await
            .get(pkey)
            .context(NoSuchPeer { remote: *pkey })?
            .lock()
            .await
            .send(message.deref())
            .await
            .context(ConnectionError { remote: *pkey })
    }

    /// Add a new `ConnectionWrite` to this `Sender`
    async fn add_connection(&self, write: ConnectionWrite) {
        if let Some(conn) = self
            .connections
            .write()
            .await
            .insert(*write.remote_pkey(), Mutex::new(write))
        {
            let pkey = *conn.lock().await.remote_pkey();
            warn!("replaced connection to {}, messages may be dropped", pkey);
        }
    }

    async fn keys(&self) -> Vec<PublicKey> {
        self.connections.read().await.keys().copied().collect()
    }
}

/// A `Sender` that uses an input messages type I and implements an output `Sender`
/// using the `Into` trait
pub struct ConvertSender<I, O, S>
where
    I: Message + 'static + Into<O>,
    O: Message + 'static,
    S: Sender<O>,
{
    sender: Arc<S>,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
}

impl<I, O, S> ConvertSender<I, O, S>
where
    I: Message + 'static + Into<O>,
    O: Message + 'static,
    S: Sender<O>,
{
    /// Create a new `ConvertSender` using the provided underlying `Sender`
    /// to actually send messages
    pub fn new(sender: Arc<S>) -> Self {
        Self {
            sender,
            _i: PhantomData,
            _o: PhantomData,
        }
    }
}

#[async_trait]
impl<I, O, S> Sender<I> for ConvertSender<I, O, S>
where
    I: Message + 'static + Into<O>,
    O: Message + 'static,
    S: Sender<O>,
{
    async fn send(
        &self,
        message: Arc<I>,
        to: &PublicKey,
    ) -> Result<(), SenderError> {
        let message = message.deref().clone().into();

        self.sender.send(Arc::new(message), to).await
    }

    async fn keys(&self) -> Vec<PublicKey> {
        self.sender.keys().await
    }

    async fn add_connection(&self, write: ConnectionWrite) {
        self.sender.add_connection(write).await
    }
}

/// A `Sender` that only collects messages instead of sending them
pub struct CollectingSender<M: Message> {
    messages: Mutex<Vec<(PublicKey, Arc<M>)>>,
    keys: Mutex<HashSet<PublicKey>>,
}

impl<M: Message> CollectingSender<M> {
    /// Create a new `CollectingSender` using a specified set of `PublicKey`
    /// destinations
    pub fn new<I: IntoIterator<Item = PublicKey>>(keys: I) -> Self {
        Self {
            messages: Mutex::new(Vec::new()),
            keys: Mutex::new(keys.into_iter().collect()),
        }
    }

    /// Retrieve the set of messages that was sent using this `CollectingSender`
    pub async fn messages(&self) -> Vec<(PublicKey, Arc<M>)> {
        self.messages.lock().await.iter().cloned().collect()
    }
}

#[async_trait]
impl<M: Message + 'static> Sender<M> for CollectingSender<M> {
    async fn send(
        &self,
        message: Arc<M>,
        key: &PublicKey,
    ) -> Result<(), SenderError> {
        ensure!(
            self.keys.lock().await.contains(key),
            NoSuchPeer { remote: *key }
        );

        self.messages.lock().await.push((*key, message));

        Ok(())
    }

    async fn add_connection(&self, write: ConnectionWrite) {
        self.keys.lock().await.insert(*write.remote_pkey());
    }

    async fn keys(&self) -> Vec<PublicKey> {
        self.keys.lock().await.clone().iter().copied().collect()
    }
}
