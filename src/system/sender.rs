use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;
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
    NoSuchPeer {
        /// The peer we attempted to send to
        remote: PublicKey,
    },
    #[snafu(display("connection with {} is broken: {}", remote, source))]
    /// The `Connection` encountered an error while sending
    ConnectionError {
        /// The peer we were trying to send to
        remote: PublicKey,
        /// Actual cause of the error
        source: SendError,
    },
    #[snafu(display("{} send errors", errors.len()))]
    /// Many send errors were encountered
    ManyErrors {
        /// All encountered errors when sending multiple messages
        errors: Vec<SenderError>,
    },
}

#[async_trait]
/// Trait used when sending messages out from `Processor`s.
pub trait Sender<M: Message + 'static>: Send + Sync {
    /// Add a new `ConnectionWrite` to this `Sender`
    async fn add_connection(&self, write: ConnectionWrite);

    /// Remove a connection by `PublicKey` from this `Sender`
    async fn remove_connection(&self, key: &PublicKey);

    /// Get the keys of  peers known by this `Sender`
    ///
    /// # Returns
    /// A `Vec` containing the keys of all peers to which there is an outbound `Connection`
    /// at this time.
    async fn keys(&self) -> Vec<PublicKey>;

    /// Send a message to a given peer using this `Sender`
    async fn send(
        &self,
        message: M,
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
        I: IntoIterator<Item = M> + Send,
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
        S: Stream<Item = M> + Send,
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
        message: M,
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
        message: M,
        pkey: &PublicKey,
    ) -> Result<(), SenderError> {
        self.connections
            .read()
            .await
            .get(pkey)
            .context(NoSuchPeer { remote: *pkey })?
            .lock()
            .await
            .send(&message)
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

    async fn remove_connection(&self, key: &PublicKey) {
        self.connections.write().await.remove(key);
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

    /// Return the inner sender wrapped by this `ConvertSender`
    pub fn into_inner(self) -> Arc<S> {
        self.sender
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
        message: I,
        to: &PublicKey,
    ) -> Result<(), SenderError> {
        self.sender.send(message.into(), to).await
    }

    async fn keys(&self) -> Vec<PublicKey> {
        self.sender.keys().await
    }

    async fn add_connection(&self, write: ConnectionWrite) {
        self.sender.add_connection(write).await
    }

    async fn remove_connection(&self, key: &PublicKey) {
        self.sender.remove_connection(key).await;
    }
}

/// A `Sender` that only collects messages instead of sending them
pub struct CollectingSender<M: Message> {
    messages: Mutex<Vec<(PublicKey, M)>>,
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
    pub async fn messages(&self) -> Vec<(PublicKey, M)> {
        self.messages.lock().await.iter().cloned().collect()
    }
}

#[async_trait]
impl<M: Message + 'static> Sender<M> for CollectingSender<M> {
    async fn send(
        &self,
        message: M,
        key: &PublicKey,
    ) -> Result<(), SenderError> {
        ensure!(
            self.keys.lock().await.contains(key),
            NoSuchPeer { remote: *key }
        );

        self.messages.lock().await.push((*key, message));

        Ok(())
    }

    async fn remove_connection(&self, key: &PublicKey) {
        self.keys.lock().await.remove(key);
    }

    async fn add_connection(&self, write: ConnectionWrite) {
        self.keys.lock().await.insert(*write.remote_pkey());
    }

    async fn keys(&self) -> Vec<PublicKey> {
        self.keys.lock().await.clone().iter().copied().collect()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::system::message;
    use crate::test::keyset;

    use serde::{Deserialize, Serialize};

    #[tokio::test]
    async fn convert_sender() {
        #[message]
        #[derive(Copy)]
        struct M1(u8);

        #[message]
        #[derive(Copy)]
        struct M2(u16);

        impl From<M1> for M2 {
            fn from(v: M1) -> Self {
                Self(v.0.into())
            }
        }

        const COUNT: u16 = 10;

        let expected = (0..COUNT).map(M2).collect::<Vec<_>>();

        let peer = keyset(1).next().unwrap();
        let sender = CollectingSender::<M2>::new(vec![peer]);

        let sender = ConvertSender::new(Arc::new(sender));

        sender
            .send_many_to_one(expected.iter().cloned(), &peer)
            .await
            .expect("send failed");

        let sender = sender.into_inner();

        let messages = sender.messages().await.into_iter().map(Into::into);

        assert_eq!(messages.len(), COUNT.into(), "wrong message count");

        messages
            .map(|x: (PublicKey, M2)| x.1)
            .zip(expected.into_iter().map(Into::into))
            .for_each(|(a, b)| assert_eq!(a, b, "bad message"));
    }
}
