use std::{
    fmt,
    io::{Error, ErrorKind},
    net::SocketAddr,
    time::Duration,
};

use async_trait::async_trait;
use snafu::{ResultExt, Snafu};
use tokio::{
    net::ToSocketAddrs,
    sync::oneshot::{channel, Receiver, Sender},
    task::{self, JoinHandle},
    time::{interval, Interval},
};
use tracing::{error, info, trace_span};
use tracing_futures::Instrument;

use super::{
    super::{
        common::directory::{Request, Response},
        connector::{ConnectError, Connector},
        socket::Socket,
        utils::resolve_addr,
        Connection, ReceiveError,
    },
    *,
};
use crate::crypto::key::exchange::{Exchanger, PublicKey};

#[derive(Debug, Snafu)]
enum DirectoryError {
    #[snafu(display("protocol error: {}", reason))]
    Protocol { reason: String },

    #[snafu(display("network error: {}", source))]
    Network { source: ReceiveError },
}

/// A `Listener` that registers its local address with a given directory server.
pub struct DirectoryListener {
    listener: Box<dyn Listener<Candidate = SocketAddr>>,
    directory_addr: SocketAddr,
    exit_tx: Sender<()>,
}

impl DirectoryListener {
    /// Create a new `DirectoryListener` that will listen for incoming
    /// connection on the given address.
    ///
    /// # Arguments
    /// * `listener` The `Listener` to accept `Connection`s with
    /// * `connector` The `Connector` to use when connecting to the directory
    /// * `directory` The `Candidate`s used for reaching the directory server
    ///
    /// # Example
    /// ```
    /// # use std::net::SocketAddr;
    /// use drop::crypto::key::exchange::{Exchanger, KeyPair};
    /// use drop::net::{DirectoryListener, ListenerError, TcpConnector, TcpListener};
    ///
    /// # async fn doc() -> Result<(), ListenerError> {
    /// let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    /// let exchanger = Exchanger::random();
    /// let listener = TcpListener::new(addr, exchanger.clone()).await?;
    /// let connector = TcpConnector::new(exchanger);
    /// let dir_pubkey = *KeyPair::random().public();
    /// let dir_addr = "example.com:80";
    /// let mut listener = DirectoryListener::new(listener, connector, dir_addr)
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub async fn new<A, C, L>(
        listener: L,
        connector: C,
        directory: A,
    ) -> Result<Self, ListenerError>
    where
        A: ToSocketAddrs + fmt::Display,
        C: Connector<Candidate = SocketAddr> + 'static,
        L: Listener<Candidate = SocketAddr> + 'static,
    {
        let directory_addr = resolve_addr(directory).await.context(Io)?;
        let (exit_tx, exit_rx) = channel();
        let listener = Box::new(listener);
        let connector = Box::new(connector);

        let mut listener = Self {
            listener,
            directory_addr,
            exit_tx,
        };

        listener
            .register(connector, directory_addr, exit_rx)
            .instrument(trace_span!("register"))
            .await?;

        Ok(listener)
    }

    /// Register ourselves on the directory server.
    /// This function will register this `Listener`'s address with the
    /// directory server.
    /// This will also schedule a task that will periodically renew the entry
    /// in the directory to prevent us being evicted.
    ///
    /// # Arguments
    /// `connector` The `Connector` used when connecting to directory
    /// `directory` Address of the directory server
    /// `exit_rx` The receiving of the channel for exit notice
    async fn register(
        &mut self,
        mut connector: Box<dyn Connector<Candidate = SocketAddr>>,
        directory: SocketAddr,
        mut exit_rx: Receiver<()>,
    ) -> Result<JoinHandle<()>, ListenerError> {
        let local = self
            .listener
            .local_addr()
            .ok_or_else(|| {
                Error::new(ErrorKind::AddrNotAvailable, "local address unknown")
            })
            .context(Io)?;
        let self_pkey = *self.listener.exchanger().keypair().public();

        Ok(task::spawn(
            async move {
                let req = Request::Add((self_pkey, local).into());
                let mut connection = connector
                    .connect(&self_pkey, &directory)
                    .instrument(trace_span!("connect"))
                    .await
                    .expect("failed to connect to directory");
                let duration = Duration::from_secs(600);
                let mut timer = interval(duration);

                info!("connected to directory!");

                loop {
                    if exit_rx.try_recv().is_ok() {
                        info!("listener is dead, stopping renewal");
                        return;
                    }
                    send_request(
                        &mut connection,
                        req,
                        connector.as_mut(),
                        &self_pkey,
                        directory,
                    )
                    .await;
                    info!("registering with directory server");
                    let resp = connection.receive_plain::<Response>().await;

                    if handle_response(resp, &mut timer, &duration)
                        .await
                        .is_err()
                    {
                        continue;
                    }
                }
            }
            .instrument(
                trace_span!("directory_renew", local=%local, server=%directory),
            ),
        ))
    }

    /// Close this `Listener` and stops the renewing of the directory entry.
    pub async fn close(self) {
        let _ = self.exit_tx.send(());
    }
}

async fn send_request(
    connection: &mut Connection,
    req: Request,
    connector: &mut dyn Connector<Candidate = SocketAddr>,
    pkey: &PublicKey,
    directory: SocketAddr,
) {
    const RETRY_DELAY: u64 = 5;

    let retry_delay = Duration::from_secs(RETRY_DELAY);
    let mut timer = interval(retry_delay);

    if let Err(e) = connection.send_plain(&req).await {
        error!("failed to send message: {}", e);

        while let Err(e) =
            check_connection(connector, connection, pkey, directory).await
        {
            error!("failed to re-establish connection to directory: {}", e);
            timer.tick().await;
        }
    }
}

async fn check_connection(
    connector: &mut dyn Connector<Candidate = SocketAddr>,
    connection: &mut Connection,
    pkey: &PublicKey,
    dir_addr: SocketAddr,
) -> Result<(), ConnectError> {
    error!("lost connection to directory, reconnecting");

    *connection = connector.connect(pkey, &dir_addr).await?;

    Ok(())
}

async fn handle_response(
    resp: Result<Response, ReceiveError>,
    timer: &mut Interval,
    duration: &Duration,
) -> Result<(), DirectoryError> {
    let resp = resp.context(Network)?;

    match resp {
        Response::Ok => {
            info!(
                "renewed lease successfully, next renew in {} seconds",
                duration.as_secs(),
            );
            timer.tick().await;
            Ok(())
        }
        other => Protocol {
            reason: format!("expected Response::Ok response got {}", other),
        }
        .fail(),
    }
}

#[async_trait]
impl Listener for DirectoryListener {
    type Candidate = DirectoryCandidate;

    async fn establish(&mut self) -> Result<Box<dyn Socket>, ListenerError> {
        Ok(self.listener.establish().await?)
    }

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr()
    }

    fn exchanger(&self) -> &Exchanger {
        self.listener.exchanger()
    }

    async fn candidates(&self) -> Result<Vec<Self::Candidate>, ListenerError> {
        Ok(vec![DirectoryCandidate::new(
            self.directory_addr,
            *self.exchanger().keypair().public(),
        )])
    }
}

impl fmt::Display for DirectoryListener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "directory listener at {}", self.local_addr().unwrap(),)
    }
}

#[derive(Copy, Clone, Debug)]
/// Candidate for `DirectoryListener` that contains the address of the directory
/// as well as the `PublicKey` to look for in the `DirectoryServer`
pub struct DirectoryCandidate {
    dir_addr: SocketAddr,
    local_key: PublicKey,
}

impl DirectoryCandidate {
    pub fn new(dir_addr: SocketAddr, local_key: PublicKey) -> Self {
        Self {
            dir_addr,
            local_key,
        }
    }
}

impl fmt::Display for DirectoryCandidate {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "directory at {} with key {}",
            self.dir_addr, self.local_key
        )
    }
}

#[cfg(test)]
mod test {
    use tokio::task;

    use super::*;
    use crate::{
        crypto::key::exchange::Exchanger,
        net::{Connector, Listener, TcpConnector, TcpListener},
        test::*,
    };

    #[tokio::test]
    async fn directory_register() {
        init_logger();
        let dir_server = next_test_ip4();
        let list_addr = next_test_ip4();
        let server_exchanger = Exchanger::random();
        let srv_pub = *server_exchanger.keypair().public();
        let dir_listener =
            TcpListener::new(list_addr, server_exchanger.clone())
                .await
                .expect("listen failed");

        let handle = task::spawn(async move {
            let exchanger = Exchanger::random();
            let mut listener = TcpListener::new(dir_server, exchanger)
                .await
                .expect("listen failed");

            let mut connection =
                listener.accept().await.expect("accept failed");

            let request = connection
                .receive_plain::<Request>()
                .await
                .expect("read request failed");

            assert_eq!(
                request,
                Request::Add((srv_pub, list_addr).into()),
                "bad request"
            );

            connection
                .send_plain(&Response::Ok)
                .await
                .expect("response failed");

            let connector = TcpConnector::new(Exchanger::random());

            connector
                .connect(&srv_pub, &list_addr)
                .await
                .expect("connect failed");
        });

        let connector = TcpConnector::new(server_exchanger);
        let mut listener =
            DirectoryListener::new(dir_listener, connector, dir_server)
                .await
                .expect("dir_bind failed");

        listener.accept().await.expect("accept failed");

        handle.await.expect("dir server failed");
    }
}
