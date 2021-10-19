use std::{iter, marker::PhantomData, sync::Arc};

use futures::{
    stream::{self, FuturesUnordered, StreamExt},
    FutureExt as _,
};
use postage::{dispatch, mpsc, sink::Sink, stream::Stream};
use snafu::OptionExt;
use tokio::task::{self, JoinHandle};
use tracing::{debug, debug_span, error, info, warn};
use tracing_futures::Instrument;

use super::{sender::NetworkSender, Sampler, Sender, System};
use crate::{
    Message,
    async_trait,
    crypto::key::exchange::PublicKey,
    net::{Connection, ConnectionRead, ConnectionWrite},
};

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
    type Error: std::error::Error + Send + Sync;

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
    incoming: Box<dyn futures::Stream<Item = Connection> + Send + Unpin>,
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
    ///
    /// # Arguments
    ///
    /// - `processor`: The [`Processor`] that will be used to process incoming messages
    /// - `sampler`: A [`Sampler`] that probabilistic algorithms will use to sample the set of peers
    /// - `parallelism`: The maximum amount of messages that will be processed in parallel
    pub async fn run<S, P, O, I, H>(
        self,
        mut processor: P,
        sampler: S,
        parallelism: usize,
    ) -> SystemHandle<P, NetworkSender<M>, I, O, M>
    where
        S: Sampler,
        P: Processor<M, I, O, NetworkSender<M>, Handle = H> + 'static,
        P::Error: 'static,
        O: Send,
        I: Send,
        M: From<I>,
        H: Handle<I, O>,
    {
        info!("beginning system setup");

        let sampler = Arc::new(sampler);
        let sender = Arc::new(NetworkSender::new(self.writes));
        let sender_add = sender.clone();

        let (user_connection_tx, user_connection_rx) = mpsc::channel(1);
        let connection_input =
            vec![self.incoming, Box::new(user_connection_rx)];
        let mut incoming = stream::select_all(connection_input);
        let (msg_tx, msg_rx) = dispatch::channel(128);
        let (error_tx, error_rx) = dispatch::channel(32);
        let (mut connection_tx, connection_rx) = mpsc::channel(16);

        let perr_tx = error_tx.clone();

        let handles = Self::spawn_network_agents(self.reads, msg_tx.clone())
            .collect::<FuturesUnordered<_>>();

        Self::spawn_disconnect_watcher::<P, _, _, _, _>(
            handles,
            msg_tx,
            error_tx.clone(),
            connection_rx,
        );

        let handle = processor.setup(sampler, sender.clone()).await;
        let processor = Arc::new(processor);

        debug!("setting up processing tasks...");

        (0..parallelism)
            .zip(iter::repeat((processor.clone(), msg_rx, sender, perr_tx)))
            .map(|(idx, (processor, mut msg_rx, sender, mut err_tx))| {
                task::spawn(async move {
                    while let Some((pkey, message)) = msg_rx.recv().await {
                        debug!("starting processing for {:?} from {}", message, pkey);

                        if let Err(e) = processor.process(message, pkey, sender.clone()).await {
                            error!("failed to process message: {}", e);

                            let error = SystemError::ProcessorError { source: e };

                            let _ = err_tx.send(error).await;
                        }
                    }

                    warn!("message processing ending after all network agents closed");
                }.instrument(debug_span!("process_task", idx=%idx)))
            }).for_each(drop); // we want to process the whole iterator but not keep the handles

        // spawn new connection handler
        task::spawn(async move {
            while let Some(connection) = incoming.next().await {
                if let Some((read, write)) = connection.split() {
                    info!(
                        "new incoming connection from {}",
                        write.remote_pkey()
                    );
                    sender_add.add_connection(write).await;

                    let _ = connection_tx.send(read).await;
                }
            }
        });

        info!("done setting up! system now running");

        SystemHandle::new(processor, handle, user_connection_tx, error_rx)
    }

    fn spawn_network_agents<I, S>(
        reads: I,
        sink: S,
    ) -> impl Iterator<Item = JoinHandle<PublicKey>>
    where
        I: IntoIterator<Item = ConnectionRead>,
        S: Sink<Item = (PublicKey, M)> + Send + Clone + Sync + Unpin + 'static,
    {
        debug!("spawning networking agents...");

        reads
            .into_iter()
            .zip(iter::repeat(sink))
            .map(|(read, tx)| Self::spawn_receive_agent(read, tx))
    }

    fn spawn_disconnect_watcher<P, E, D, R, ER>(
        mut receivers: FuturesUnordered<JoinHandle<PublicKey>>,
        msg_dispatch: D,
        mut error_tx: E,
        mut connection_rx: R,
    ) where
        ER: std::error::Error + Send + Sync + 'static,
        E: Sink<Item = SystemError<ER>> + Send + Unpin + 'static,
        D: Sink<Item = (PublicKey, M)> + Clone + Sync + Send + Unpin + 'static,
        R: Stream<Item = ConnectionRead> + Send + Unpin + 'static,
    {
        debug!("spawning disconnect watcher...");

        task::spawn(async move {
            while !receivers.is_empty() {
                futures::select! {
                    // new connection to be added to list of receivers
                    read = connection_rx.recv().fuse() => {

                        if let Some(read) = read {
                            debug!("new incoming connection");

                            receivers.push(NetworkAgent::new(read, msg_dispatch.clone()).spawn());
                        }
                    }
                    // disconnection notice
                    pkey = receivers.next() => {
                        let pkey = pkey.unwrap().unwrap();


                        if error_tx.send(Disconnected { pkey }.build()).await.is_err() {
                            error!("error handle dropped too early some errors were lost");
                        }
                    }
                }
            }
        });
    }

    fn spawn_receive_agent<S>(
        connection: ConnectionRead,
        tx: S,
    ) -> JoinHandle<PublicKey>
    where
        S: Sink<Item = (PublicKey, M)> + Send + Sync + Unpin + 'static,
    {
        NetworkAgent::new(connection, tx).spawn()
    }
}

#[derive(Debug, snafu::Snafu)]
/// Errors encountered by [`SystemHandle`]
///
/// [`SystemHandle`]: self::SystemHandle
pub enum SystemError<E: std::error::Error + Send + Sync + 'static> {
    #[snafu(display("unauthenticated connection"))]
    /// User tried to add an unauthenticated connection
    Unauthenticated,
    #[snafu(display("remote peer {} disconnected", pkey))]
    /// A connection error caused a remote peer to be disconnected
    Disconnected {
        /// Peer's PublicKey
        pkey: PublicKey,
    },
    #[snafu(display("processor error: {}", source))]
    /// Processor encountered an error
    ProcessorError {
        /// Error source
        source: E,
    },
    #[snafu(display("connection channel is closed"))]
    /// Connection channel was closed and the connection could not be added.
    /// Adding further connections will not work either
    Channel,
}

/// This is handle used to interact with a [`SystemManager`] and the [`Processor`]
/// running on that [`SystemManager`]
///
/// [`Processor`]: self::Processor
/// [`SystemManager`]: self::SystemManager
#[derive(Clone)]
pub struct SystemHandle<P, S, I, O, M>
where
    P: Processor<M, I, O, S>,
    P::Error: Send + Sync + 'static,
    O: Send,
    M: Message + From<I> + 'static,
    S: Sender<M>,
{
    inner: P::Handle,
    processor: Arc<P>,
    connections: mpsc::Sender<Connection>,
    error_rx: Option<dispatch::Receiver<SystemError<P::Error>>>,
    _i: PhantomData<I>,
    _o: PhantomData<O>,
}

impl<P, S, I, O, M> SystemHandle<P, S, I, O, M>
where
    P: Processor<M, I, O, S> + Send,
    P::Error: Send + Sync + 'static,
    I: Send,
    O: Send,
    M: Message + From<I> + 'static,
    S: Sender<M>,
{
    fn new(
        processor: Arc<P>,
        inner: P::Handle,
        connections: mpsc::Sender<Connection>,
        error_rx: dispatch::Receiver<SystemError<P::Error>>,
    ) -> Self {
        Self {
            inner,
            processor,
            connections,
            error_rx: Some(error_rx),
            _i: PhantomData,
            _o: PhantomData,
        }
    }

    /// Get [`Handle`] for the [`Processor`] currently running
    ///
    /// [`Handle`]: self::Handle
    /// [`Processor`]: self::Processor
    pub fn processor_handle(&self) -> P::Handle {
        self.inner.clone()
    }

    /// Force garbage collection of the [`Processor`]
    ///
    /// [`Processor`]: self::Processor
    pub async fn force_gc(&self) {
        self.processor.garbage_collection().await;
    }

    /// Get a `Stream` that will yield all errors encountered in the running [`SystemManager`]
    ///
    /// # Note
    /// The `Stream` of errors can only be taken once, further calls will only return `None`
    ///
    /// [`SystemManager`]: self::SystemManager
    pub fn errors(
        &mut self,
    ) -> Option<impl futures::Stream<Item = SystemError<P::Error>>> {
        self.error_rx.take()
    }

    /// Add a new [`Connection`] to the running [`SystemManager`]
    ///
    /// [`Connection`]: crate::net::Connection
    /// [`SystemManager`]: self::SystemManager
    pub async fn add_connection(
        &self,
        connection: Connection,
    ) -> Result<(), SystemError<P::Error>> {
        let key = connection.remote_key().context(Unauthenticated)?;

        debug!("adding connection from user to {}", key);

        self.connections
            .clone()
            .send(connection)
            .await
            .ok()
            .context(Channel)?;

        Ok(())
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
    use std::{
        error::Error,
        sync::atomic::{AtomicUsize, Ordering},
    };

    use tokio::sync::{mpsc, Mutex};

    use super::{super::sampler::AllSampler, *};
    use crate::test::*;

    #[derive(Default)]
    struct Dummy {
        sender: Option<mpsc::Sender<(PublicKey, usize)>>,
    }

    #[derive(Debug)]
    struct UnreachableError;
    impl std::fmt::Display for UnreachableError {
        fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            Err(std::fmt::Error)
        }
    }
    impl Error for UnreachableError {}

    #[async_trait]
    impl Processor<usize, usize, (PublicKey, usize), NetworkSender<usize>>
        for Dummy
    {
        type Handle = TestHandle<usize>;

        type Error = UnreachableError;

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
        type Error = UnreachableError;

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

        let (pkeys, handles, system) =
            create_system(COUNT, |mut connection| async move {
                let value = COUNTER.fetch_add(1, Ordering::AcqRel);

                debug!("sending {:?}", value);
                connection.send(&value).await.expect("recv failed");

                debug!("done sending");
            })
            .await;

        let sampler = AllSampler::default();
        let processor = Dummy::default();
        let manager = SystemManager::new(system);

        debug!("manager created");

        debug!("registering processor");

        let system_handle = manager.run(processor, sampler, 1).await;
        let mut handle = system_handle.processor_handle();

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

    #[tokio::test]
    async fn disconnect_notice() {
        static COUNT: usize = 50;

        use std::collections::HashSet;

        crate::test::init_logger();

        let (pkeys, handles, system) = create_system(COUNT, |_| async {}).await;

        let manager = SystemManager::<usize>::new(system);
        let processor = Dummy::default();

        let mut system_handle =
            manager.run(processor, AllSampler::default(), 1).await;

        let source = system_handle.errors().unwrap();

        let actual = source
            .map(|x| match x {
                SystemError::Disconnected { pkey } => pkey,
                e => panic!("bad error type: {}", e),
            })
            .collect::<HashSet<_>>()
            .await;

        assert_eq!(actual.len(), COUNT, "wrong number of disconnect notice");

        let expected = pkeys.into_iter().map(|x| x.0).collect::<HashSet<_>>();

        assert_eq!(actual, expected, "missing some disconnect notices");

        handles.await.expect("system failure");
    }
}
