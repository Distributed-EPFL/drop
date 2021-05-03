use std::iter;
use std::marker::PhantomData;
use std::sync::Arc;

use super::sender::NetworkSender;
use super::{Message, Sampler, Sender, System};

use crate::async_trait;
use crate::crypto::key::exchange::PublicKey;
use crate::net::{Connection, ConnectionRead, ConnectionWrite, ReceiveError};

use futures::stream::FuturesUnordered;
use futures::{Stream, StreamExt};

use postage::dispatch;
use postage::sink::Sink;
use postage::stream::Stream as _;

use tokio::task::{self, JoinHandle};

use tracing::{debug, error, info, warn};

#[async_trait]
/// Trait used to process incoming messages from a `SystemManager`
///
/// [`SystemManager`]: self::SystemManager
pub trait Processor<M, I, O, S>: Send + Sync
where
    M: Message + 'static,
    I: Into<M>,
    O: Send,
    S: Sender<M>,
{
    /// The [`Handle`] used to send and receive messages from the `Processor`
    ///
    /// [`Handle`]: self::Handle
    type Handle: Handle<I, O>;

    /// Type of errors returned by `Processor::process`
    type Error: std::error::Error;

    /// Process an incoming message using this `Processor`
    async fn process(
        &self,
        message: M,
        from: PublicKey,
        sender: Arc<S>,
    ) -> Result<(), Self::Error>;

    /// Setup the `Processor` using the given sender map and returns a `Handle`
    /// for the user to use.
    async fn setup<SA: Sampler>(
        &mut self,
        sampler: Arc<SA>,
        sender: Arc<S>,
    ) -> Self::Handle;

    /// Used by managers to signal a disconnection to the `Processor` allowing it to resample if needed
    async fn disconnect<SA: Sampler>(
        &self,
        peer: PublicKey,
        sender: Arc<S>,
        sampler: Arc<SA>,
    );

    /// Called periodically by the manager to start garbage collection by the `Processor`
    async fn garbage_collection(&self);
}

/// An asbtract `Handle` type that allows interacting with a `Processor` once it
/// has been scheduled to run on a `SystemManager`. This type will usually be
/// obtained by calling SystemManager::run on a previously created `Processor`
#[async_trait]
pub trait Handle<I, O>: Send + Sync + Clone {
    /// Type of errors returned by this `Handle` type
    type Error: std::error::Error;

    /// Deliver a message using this `Handle`. <br />
    /// This method returns `Ok` if some message can be delivered or an `Err`
    /// otherwise
    async fn deliver(&mut self) -> Result<O, Self::Error>;

    /// Poll this `Handle` for delivery, returning immediately with `Ok(None)`
    /// if no message is available for delivery or `Ok(Some)` if a message is

    /// otherwise
    async fn try_deliver(&mut self) -> Result<Option<O>, Self::Error>;

    /// Starts broadcasting a message using this `Handle`
    async fn broadcast(&mut self, message: &I) -> Result<(), Self::Error>;
}

/// Handles sending and receiving messages from all known peers.
/// Also forwards them to relevant destination for processing
pub struct SystemManager<M: Message + 'static> {
    _m: PhantomData<M>,
    reads: Vec<ConnectionRead>,
    writes: Vec<ConnectionWrite>,
    /// `Stream` of incoming `Connection`s
    incoming: Box<dyn Stream<Item = Connection> + Send + Unpin>,
}

impl<M: Message + 'static> SystemManager<M> {
    /// Create a new `SystemManager` using some previously created `System`
    pub fn new(mut system: System) -> Self {
        debug!("creating manager");

        let (reads, writes): (Vec<_>, Vec<_>) = system
            .connections()
            .into_iter()
            .filter_map(|connection| connection.split())
            .unzip();

        let incoming = Box::new(system.peer_source());

        Self {
            reads,
            writes,
            incoming,
            _m: PhantomData,
        }
    }

    /// Start the `SystemManager`. <br />
    /// Provide a `Processor` that implements the algorithm you want to run
    /// as well as a `Sampler` which will determine if the probabilistic
    /// or deterministic version of the algorithm will be run. <br />
    /// This returns a `Handle` that allows interaction while the system is
    /// running
    pub async fn run<
        S: Sampler,
        P: Processor<M, I, O, NetworkSender<M>, Handle = H> + 'static,
        O: Send,
        I: Into<M>,
        H: Handle<I, O>,
    >(
        self,
        mut processor: P,
        sampler: S,
    ) -> H {
        info!("beginning system setup");

        debug!("setting up dispatcher...");

        let sampler = Arc::new(sampler);
        let sender = Arc::new(NetworkSender::new(self.writes));
        let sender_add = sender.clone();
        let mut incoming = self.incoming;

        let (msg_tx, msg_rx) = dispatch::channel(128);

        let dispatcher = msg_tx.clone();

        let handles = self
            .reads
            .into_iter()
            .zip(iter::repeat(msg_tx))
            .map(|(read, tx)| Self::spawn_receive_agent(read, tx))
            .collect::<FuturesUnordered<_>>();

        let handle = processor.setup(sampler, sender.clone()).await;
        let processor = Arc::new(processor);

        let processing_handles = (0..32)
            .zip(iter::repeat((processor, msg_rx, sender)))
            .map(|(_, (processor, msg_rx, sender))| (processor, msg_rx, sender))
            .map(|(processor, mut msg_rx, sender)| {
                task::spawn(async move {
                    while let Some((pkey, message)) = msg_rx.recv().await {
                        debug!("received {:?} from {}", message, pkey);

                        if let Err(e) = processor.process(message, pkey, sender.clone()).await {
                            error!("failed to process message: {}", e);
                        }
                    }

                    warn!("message processing ending after all network agents closed");
                })
            }).collect::<FuturesUnordered<_>>();

        // spawn new connection handler
        task::spawn(async move {
            while let Some(connection) = incoming.next().await {
                if let Some((read, write)) = connection.split() {
                    info!(
                        "new incoming connection from {}",
                        write.remote_pkey()
                    );
                    sender_add.add_connection(write).await;

                    Self::spawn_receive_agent(read, dispatcher.clone());
                }
            }
        });

        debug!("done setting up dispatcher! system now running");

        handle
    }

    fn spawn_receive_agent<S>(
        connection: ConnectionRead,
        tx: S,
    ) -> JoinHandle<Result<(), ReceiveError>>
    where
        S: Sink<Item = (PublicKey, M)> + Send + Sync + Unpin + 'static,
    {
        NetworkAgent::new(connection, tx).spawn()
    }
}

struct NetworkAgent<M, S>
where
    S: Sink<Item = (PublicKey, M)>,
{
    sender: S,
    read: ConnectionRead,
    pkey: PublicKey,
}

impl<M, S> NetworkAgent<M, S>
where
    M: Message + 'static,
    S: Sink<Item = (PublicKey, M)> + Send + Sync + Unpin + 'static,
{
    fn new(read: ConnectionRead, sender: S) -> Self {
        let pkey = *read.remote_pkey();

        Self { sender, read, pkey }
    }

    fn spawn(mut self) -> JoinHandle<PublicKey> {
        let pkey = self.pkey;

        task::spawn(
            async move { self.receive_loop().await }
                .instrument(debug_span!("network_agent", peer=%pkey)),
        )
    }

    async fn receive_loop(&mut self) -> PublicKey {
        loop {
            match self.read.receive::<M>().await {
                Err(e) => {
                    error!("connection with failed: {}", e);
                    return self.pkey;
                }
                Ok(message) => {
                    if self.sender.send((self.pkey, message)).await.is_err() {
                        warn!("network agent shutting down");
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::super::sampler::AllSampler;
    use super::*;
    use crate::test::*;

    use tokio::sync::{mpsc, Mutex};

    #[derive(Clone)]
    struct TestHandle<M>
    where
        M: Message,
    {
        channel: Arc<Mutex<mpsc::Receiver<(PublicKey, M)>>>,
    }

    #[async_trait]
    impl<M> Handle<M, (PublicKey, M)> for TestHandle<M>
    where
        M: Message,
    {
        type Error = mpsc::error::RecvError;

        async fn deliver(&mut self) -> Result<(PublicKey, M), Self::Error> {
            Ok(self.channel.lock().await.recv().await.expect("no message"))
        }

        async fn try_deliver(
            &mut self,
        ) -> Result<Option<(PublicKey, M)>, Self::Error> {
            unreachable!()
        }

        async fn broadcast(&mut self, _: &M) -> Result<(), Self::Error> {
            unreachable!()
        }
    }

    #[tokio::test]
    async fn receive_from_manager() {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        const COUNT: usize = 50;

        #[derive(Default)]
        struct Dummy {
            sender: Option<mpsc::Sender<(PublicKey, usize)>>,
        }

        #[async_trait]
        impl Processor<usize, usize, (PublicKey, usize), NetworkSender<usize>>
            for Dummy
        {
            type Handle = TestHandle<usize>;

            type Error = mpsc::error::RecvError;

            async fn process(
                &self,
                message: usize,
                key: PublicKey,
                _sender: Arc<NetworkSender<usize>>,
            ) -> Result<(), Self::Error> {
                self.sender
                    .as_ref()
                    .expect("not setup")
                    .clone()
                    .send((key, message))
                    .await
                    .expect("channel failure");

                Ok(())
            }

            async fn setup<SA: Sampler>(
                &mut self,
                _sampler: Arc<SA>,
                _sender: Arc<NetworkSender<usize>>,
            ) -> Self::Handle {
                let (tx, rx) = mpsc::channel(128);

                self.sender.replace(tx);

                let channel = Arc::new(Mutex::new(rx));

                TestHandle { channel }
            }

            async fn disconnect<SA: Sampler>(
                &self,
                _: PublicKey,
                _: Arc<NetworkSender<usize>>,
                _: Arc<SA>,
            ) {
                unreachable!()
            }

            async fn garbage_collection(&self) {
                unreachable!()
            }
        }

        let (pkeys, handles, system) =
            create_system(COUNT, |mut connection| async move {
                let value = COUNTER.fetch_add(1, Ordering::AcqRel);

                connection.send(&value).await.expect("recv failed");
            })
            .await;

        let sampler = AllSampler::default();
        let processor = Dummy::default();
        let manager = SystemManager::new(system);

        debug!("manager created");

        debug!("registering processor");

        let mut handle = manager.run(processor, sampler).await;
        let mut messages = Vec::with_capacity(COUNT);

        for _ in 0..COUNT {
            let (pkey, message) =
                handle.deliver().await.expect("unexpected error");

            assert!(
                pkeys.iter().any(|(key, _)| *key == pkey),
                "bad message sender"
            );

            messages.push(message);
        }

        messages.sort_unstable();

        assert_eq!(
            messages,
            (0..COUNT).collect::<Vec<_>>(),
            "incorrect message sequence"
        );

        handles.await.expect("system failure");
    }
}
