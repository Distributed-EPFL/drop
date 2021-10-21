use std::{
    future::Future,
    marker::PhantomData,
    net::{Ipv4Addr, SocketAddr},
    sync::{
        atomic::{AtomicU16, Ordering},
        Arc,
    },
};

use futures::{future, stream::StreamExt};
use tokio::task::{self, JoinHandle};
use tracing::{info, trace};

use super::*;
use crate::{
    crypto::key::exchange::{Exchanger, KeyPair, PublicKey},
    net::*,
    system::{AllSampler, CollectingSender, Processor, System},
    Message,
};

/// Get the next available port for testing purposes
pub fn next_test_port() -> u16 {
    static PORT_OFFSET: AtomicU16 = AtomicU16::new(0);
    const PORT_START: u16 = 9600;

    PORT_START + PORT_OFFSET.fetch_add(1, Ordering::Relaxed)
}

/// Get the next available `SocketAddr` that can be used for testing
pub fn next_test_ip4() -> SocketAddr {
    (Ipv4Addr::new(127, 0, 0, 1), next_test_port()).into()
}

/// Generate a set of `count` address and port pairs for local testing
pub fn test_addrs(count: usize) -> Vec<(Exchanger, SocketAddr)> {
    (0..count)
        .map(|_| (Exchanger::random(), next_test_ip4()))
        .collect()
}

/// Create a sent of receivers using a user provided callback
pub async fn create_receivers<
    I: Iterator<Item = (Exchanger, SocketAddr)>,
    F: Future<Output = ()> + Send + Sync,
    C: Fn(Connection) -> F + Send + Sync + Clone + 'static,
>(
    addrs: I,
    callback: C,
) -> Vec<(PublicKey, JoinHandle<()>)> {
    let mut output = Vec::new();

    for (exchanger, addr) in addrs {
        let pkey = *exchanger.keypair().public();
        let mut listener = TcpListener::new(addr, exchanger)
            .await
            .expect("listen failed");

        let callback = callback.clone();

        let handle = task::spawn(async move {
            let connection = listener.accept().await.expect("accept failed");

            info!("secure connection accepted");

            (callback)(connection).await;
        });

        output.push((pkey, handle));
    }

    output
}

/// Helper to create a `System` using a number of connection to open
/// and some user defined action once the connection has been established
pub async fn create_system<
    C: Fn(Connection) -> F + Clone + Sync + Send + 'static,
    F: Future<Output = ()> + Send + Sync,
>(
    size: usize,
    closure: C,
) -> (Vec<(PublicKey, SocketAddr)>, JoinHandle<()>, System) {
    init_logger();
    let tcp = TcpConnector::new(Exchanger::random());
    let mut addrs = test_addrs(size);
    let public =
        create_receivers(addrs.clone().into_iter(), move |connection| {
            (closure)(connection)
        })
        .await;
    let pkeys = public.iter().map(|x| x.0);
    let candidates_iter = pkeys.zip(addrs.drain(..).map(|x| x.1));
    let output = candidates_iter.collect::<Vec<_>>();
    let handle = task::spawn(async move {
        future::join_all(public.into_iter().map(|x| x.1))
            .await
            .into_iter()
            .for_each(|x| x.expect("connection failure"))
    });

    (
        output.clone(),
        handle,
        System::new_with_connector_zipped(&tcp, output).await,
    )
}

/// Generate a set of random public keys for local testing
pub fn keyset(count: usize) -> impl Iterator<Item = PublicKey> + Clone {
    (0..count).map(|_| *KeyPair::random().public())
}

/// A `SystemManager` that uses a set sequence of messages for testing
pub struct DummyManager<M: Message, O> {
    incoming: Vec<(PublicKey, M)>,
    sender: Arc<CollectingSender<M>>,
    _o: PhantomData<O>,
}

impl<M, O> DummyManager<M, O>
where
    M: Message + 'static,
    O: Send,
{
    /// Create a `DummyManager` that will deliver all specified messages
    /// from a random set of `PublicKey`s of size *count*
    pub fn new(messages: impl Iterator<Item = M>, count: usize) -> Self {
        let keys = keyset(count).collect::<Vec<_>>();

        Self::with_key(keys.clone().into_iter().cycle().zip(messages), keys)
    }

    /// Create a `DummyManager` that will deliver from a specified set of
    /// `PublicKey`
    pub fn with_key<
        I1: IntoIterator<Item = (PublicKey, M)>,
        I2: IntoIterator<Item = PublicKey>,
    >(
        messages: I1,
        keys: I2,
    ) -> Self {
        let keys = keys.into_iter();

        Self {
            sender: Arc::new(CollectingSender::new(keys)),
            incoming: messages.into_iter().collect(),
            _o: PhantomData,
        }
    }

    /// Run a `Processor` using the sequence of message specified at creation.
    /// This manager uses `PoissonSampler` internally to sample the known peers.
    pub async fn run<I, P>(&mut self, mut processor: P) -> P::Handle
    where
        I: Into<M>,
        P: Processor<M, I, O, CollectingSender<M>> + 'static,
    {
        let sampler = Arc::new(AllSampler::default());
        let handle = processor.setup(sampler, Arc::clone(&self.sender)).await;
        let processor = Arc::new(processor);
        let sender = self.sender.clone();
        let total = self.incoming.len();

        trace!("starting test processing for {} messages", total);

        let futs: futures::stream::FuturesOrdered<_> = self
            .incoming
            .drain(..)
            .enumerate()
            .map(|(idx, (key, msg))| {
                let p = processor.clone();
                let sender = sender.clone();

                async move {
                    trace!(
                        "[{}/{}] staring processing for {:?}",
                        idx + 1,
                        total,
                        msg
                    );
                    p.process(msg, key, sender).await
                }
            })
            .collect();

        futs.for_each(|x| async move {
            x.expect("processing failed");
        })
        .await;

        handle
    }

    /// Get the internal `Sender` to check what messages were sent
    pub fn sender(&self) -> Arc<CollectingSender<M>> {
        self.sender.clone()
    }
}
