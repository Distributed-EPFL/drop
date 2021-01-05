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
pub trait Processor<M, I, O, S>: Send + Sync
where
    M: Message + 'static,
    I: Message,
    O: Message,
    S: Sender<M>,
{
    /// The handle used to send and receive messages from the `Processor`
    type Handle: Handle<I, O>;

    /// Type of errors returned by `Processor::process`
    type Error: std::error::Error;

    /// Process an incoming message using this `Processor`
    async fn process(
        self: Arc<Self>,
        message: Arc<M>,
        from: PublicKey,
        sender: Arc<S>,
    ) -> Result<(), Self::Error>;

    /// Setup the `Processor` using the given sender map and returns a `Handle`
    /// for the user to use.
    async fn output<SA: Sampler>(
        &mut self,
        sampler: Arc<SA>,
        sender: Arc<S>,
    ) -> Self::Handle;
}

/// An asbtract `Handle` type that allows interacting with a `Processor` once it
/// has been scheduled to run on a `SystemManager`. This type will usually be
/// obtained by calling SystemManager::run on a previously created `Processor`
#[async_trait]
pub trait Handle<I, O>: Send + Sync {
    /// Type of errors returned by this `Handle` type
    type Error: std::error::Error;

    /// Deliver a message using this `Handle`. <br />
    /// This method returns `Ok` if some message can be delivered or an `Err`
    /// otherwise
    async fn deliver(&mut self) -> Result<O, Self::Error>;

    /// Poll this `Handle` for delivery, returning immediately with `Ok(None)`
    /// if no message is available for delivery or `Ok(Some)` if a message is
    /// ready to be delivered. `Err` is returned like `Handle::deliver`
    /// otherwise
    fn try_deliver(&mut self) -> Result<Option<O>, Self::Error>;

    /// Starts broadcasting a message using this `Handle`
    async fn broadcast(&mut self, message: &I) -> Result<(), Self::Error>;
}

/// A macro to create a `Handle` for some `Processor` and `Message` type
#[macro_export]
macro_rules! implement_handle {
    ($name:ident, $error:ident, $msg:ident) => {
        #[derive(Snafu, Debug)]
        /// Error type for $name
        pub enum $error {
            #[snafu(display("this handle is not a sender handle"))]
            /// Not a sender handle
            NotASender,

            #[snafu(display("associated sender was destroyed"))]
            /// The sender associatex with this handle doesn't exist anymore
            SenderDied,

            #[snafu(display("this handle was already used once"))]
            /// The handle was already used to broadcast or deliver
            AlreadyUsed,

            #[snafu(display("unable to deliver a message"))]
            /// No message could be delivered from this `Handle`
            NoMessage,
        }

        /// A `Handle` used to interact with a `Processor`
        pub struct $name<M: Message> {
            incoming: Option<tokio::sync::oneshot::Receiver<M>>,
            outgoing: Option<tokio::sync::oneshot::Sender<(M, Signature)>>,
            signer: Signer,
        }

        impl<M: Message> $name<M> {
            fn new(
                keypair: Arc<KeyPair>,
                incoming: oneshot::Receiver<M>,
                outgoing: Option<oneshot::Sender<(M, Signature)>>,
            ) -> Self {
                Self {
                    signer: Signer::new(keypair.deref().clone()),
                    incoming: Some(incoming),
                    outgoing,
                }
            }
        }

        #[async_trait]
        impl<M: Message> Handle<M, M> for $name<M> {
            type Error = $error;

            /// Deliver a `Message` using the algorithm associated with this
            /// `$name`. Since this is a one-shot algorithm, a `$name` can only
            /// deliver one message.
            /// All subsequent calls to this method will return `None`
            async fn deliver(&mut self) -> Result<M, Self::Error> {
                self.incoming
                    .take()
                    .context(AlreadyUsed)?
                    .await
                    .map_err(|_| snafu::NoneError)
                    .context(NoMessage)
            }

            /// Attempts delivery of a `Message` using the `Sieve` algorithm.
            /// This method returns `Ok(None)` immediately if no `Message` is
            /// ready for delivery. `Ok(Some(message))` if a message is ready.
            /// And finally `Err` if no message can be delivered using this
            /// handle
            fn try_deliver(&mut self) -> Result<Option<M>, Self::Error> {
                let mut deliver = self.incoming.take().context(AlreadyUsed)?;

                match deliver.try_recv() {
                    Ok(message) => Ok(Some(message)),
                    Err(oneshot::error::TryRecvError::Empty) => {
                        self.incoming.replace(deliver);
                        Ok(None)
                    }
                    _ => NoMessage.fail(),
                }
            }

            /// Broadcast a message using the associated broadcast instance. <br />
            /// This will return an error if the instance was created using
            /// `new_receiver` or if this is not the first time this method is
            /// called
            async fn broadcast(
                &mut self,
                message: &M,
            ) -> Result<(), Self::Error> {
                let sender = self.outgoing.take().context(NotASender)?;
                let signature =
                    self.signer.sign(message).expect("failed to sign message");

                sender
                    .send((message.clone(), signature))
                    .map_err(|_| snafu::NoneError)
                    .context(SenderDied)?;

                Ok(())
            }
        }
    };
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
        O: Message,
        I: Message,
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
        let (mut read_tx, read_rx) = mpsc::channel(8);

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

        let mut receiver = Self::new_receive(self.reads, read_rx);

        let handle = processor.output(sampler, sender.clone()).await;
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
        mut tx: mpsc::Sender<(PublicKey, Arc<M>)>,
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
    use super::super::sampler::AllSampler;
    use super::super::test::*;
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};

    #[async_trait]
    impl<M: Message> Handle<M, (PublicKey, M)> for mpsc::Receiver<(PublicKey, M)> {
        type Error = mpsc::error::RecvError;

        async fn deliver(&mut self) -> Result<(PublicKey, M), Self::Error> {
            Ok(self.recv().await.expect("no message"))
        }

        fn try_deliver(
            &mut self,
        ) -> Result<Option<(PublicKey, M)>, Self::Error> {
            unreachable!()
        }

        async fn broadcast(&mut self, _: &M) -> Result<(), Self::Error> {
            unreachable!()
        }
    }

    #[tokio::test(threaded_scheduler)]
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
            type Handle = mpsc::Receiver<(PublicKey, usize)>;

            type Error = mpsc::error::RecvError;

            async fn process(
                self: Arc<Self>,
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

            async fn output<SA: Sampler>(
                &mut self,
                _sampler: Arc<SA>,
                _sender: Arc<NetworkSender<usize>>,
            ) -> Self::Handle {
                let (tx, rx) = mpsc::channel(128);

                self.sender.replace(tx);

                rx
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
