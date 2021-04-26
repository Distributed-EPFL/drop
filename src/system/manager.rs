use std::marker::PhantomData;
use std::sync::Arc;

use super::sender::NetworkSender;
use super::{Message, Sampler, Sender, System};

use crate::async_trait;
use crate::crypto::key::exchange::PublicKey;
use crate::net::{Connection, ConnectionRead, ConnectionWrite};

use futures::{Stream, StreamExt};

use tokio::sync::mpsc;
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
        message: Arc<M>,
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
        let (read_tx, read_rx) = mpsc::channel(8);

        task::spawn(async move {
            while let Some(connection) = incoming.next().await {
                if let Some((read, write)) = connection.split() {
                    info!(
                        "new incoming connection from {}",
                        write.remote_pkey()
                    );
                    sender_add.add_connection(write).await;

                    if let Err(e) = read_tx.send(read).await {
                        error!("receive task isn't running anymore: {}", e);
                        return;
                    }
                }
            }
        });

        // FIXME: pending inclusion of `Stream` in libstd
        use tokio_stream::wrappers::ReceiverStream;
        let mut receiver =
            Self::new_receive(self.reads, ReceiverStream::new(read_rx));

        let handle = processor.setup(sampler, sender.clone()).await;
        let processor = Arc::new(processor);

        task::spawn(async move {
            loop {
                match receiver.recv().await {
                    Some((pkey, message)) => {
                        debug!("incoming message, dispatching...");
                        let sender = sender.clone();
                        let processor = processor.clone();

                        task::spawn(async move {
                            if let Err(e) =
                                processor.process(message, pkey, sender).await
                            {
                                error!("processing error :{}", e);
                            }
                        });
                    }
                    None => {
                        warn!("no more incoming messages, dispatcher exiting");
                        break;
                    }
                }
            }
        });

        debug!("done setting up dispatcher! system now running");

        handle
    }

    fn new_receive<I, S>(
        reads: I,
        mut read_rx: S,
    ) -> mpsc::Receiver<(PublicKey, Arc<M>)>
    where
        I: IntoIterator<Item = ConnectionRead>,
        S: Stream<Item = ConnectionRead> + Unpin + Send + 'static,
    {
        let (tx, rx) = mpsc::channel(32);

        reads.into_iter().for_each(|connection| {
            Self::receive_task(connection, tx.clone());
        });

        task::spawn(async move {
            while let Some(connection) = read_rx.next().await {
                Self::receive_task(connection, tx.clone());
            }
        });

        rx
    }

    fn receive_task(
        mut connection: ConnectionRead,
        tx: mpsc::Sender<(PublicKey, Arc<M>)>,
    ) -> JoinHandle<()> {
        task::spawn(async move {
            let remote = *connection.remote_pkey();

            loop {
                match connection.receive().await {
                    Ok(msg) => {
                        if tx.send((remote, Arc::new(msg))).await.is_err() {
                            info!("manager is not running anymore exiting");
                            return;
                        }
                    }
                    Err(e) => {
                        error!("receive error: {}", e);
                        return;
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use super::super::sampler::AllSampler;
    use super::*;
    use crate::test::*;

    use tokio::sync::Mutex;

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
                message: Arc<usize>,
                key: PublicKey,
                _sender: Arc<NetworkSender<usize>>,
            ) -> Result<(), Self::Error> {
                self.sender
                    .as_ref()
                    .expect("not setup")
                    .clone()
                    .send((key, *message))
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
