use std::fmt;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::time::Duration;

use super::super::common::directory::{DirectoryRequest, DirectoryResponse};
use super::super::connector::Connector;
use super::super::socket::Socket;
use super::super::utils::resolve_addr;
use super::super::{Connection, ReceiveError};
use super::*;
use crate::crypto::key::exchange::{Exchanger, PublicKey};

use async_trait::async_trait;

use snafu::ResultExt;

use tokio::net::ToSocketAddrs;
use tokio::sync::oneshot::{channel, Receiver, Sender};
use tokio::task::{self, JoinHandle};
use tokio::time::{interval, Interval};

use tracing::{error, info, trace_span};
use tracing_futures::Instrument;

/// A `Listener` that registers its local address with a given directory server.
pub struct DirectoryListener {
    listener: Box<dyn Listener<Candidate = SocketAddr>>,
    exit_tx: Sender<()>,
}

impl DirectoryListener {
    /// Create a new `DirectoryListener` that will listen for incoming
    /// connection on the given address.
    ///
    /// # Arguments
    /// * `listener` The `Listener` to accept `Connection`s with
    /// * `connector` The `Connector` to use when connecting to the directory
    /// * `dir_addr` The `Candidate`s used for reaching the directory server
    ///
    /// # Example
    /// ```
    /// # use std::net::SocketAddr;
    /// use drop::crypto::key::exchange::{Exchanger, KeyPair};
    /// use drop::net::listener::{DirectoryListener, ListenerError, TcpListener};
    /// use drop::net::connector::TcpDirect;
    ///
    /// # async fn doc() -> Result<(), ListenerError> {
    /// let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    /// let exchanger = Exchanger::random();
    /// let listener = Box::new(TcpListener::new(addr, exchanger.clone()).await?);
    /// let connector = Box::new(TcpDirect::new(exchanger));
    /// let dir_pubkey = *KeyPair::random().public();
    /// let dir_addr = "somewhere.com:80";
    /// let mut listener = DirectoryListener::new(listener, connector, dir_addr)
    ///     .await?;
    /// # Ok(()) }
    /// ```
    pub async fn new<A: ToSocketAddrs + fmt::Display>(
        listener: Box<dyn Listener<Candidate = SocketAddr>>,
        connector: Box<dyn Connector<Candidate = SocketAddr>>,
        dir_addr: A,
    ) -> Result<Self, ListenerError> {
        let resolved = resolve_addr(dir_addr).await.context(Io)?;
        let (exit_tx, exit_rx) = channel();

        let mut listener = Self { listener, exit_tx };

        listener
            .register(connector, resolved, exit_rx)
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
    /// `dir_addr` Address of the directory server
    /// `exit_rx` The receiving of the channel for exit notice
    async fn register(
        &mut self,
        mut connector: Box<dyn Connector<Candidate = SocketAddr>>,
        dir_addr: SocketAddr,
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
                let req = DirectoryRequest::Add((self_pkey, local).into());
                let socket = connector
                    .establish(&self_pkey, &dir_addr)
                    .instrument(trace_span!("connect"))
                    .await
                    .expect("failed to connect to directory");
                let duration = Duration::from_secs(600);
                let mut timer = interval(duration);
                let mut connection = Connection::new(socket);

                info!("connected to directory!");

                loop {
                    if exit_rx.try_recv().is_ok() {
                        return;
                    }

                    // if the connection can't be established we probably moved anyway
                    check_connection(
                        connector.as_mut(),
                        &mut connection,
                        &self_pkey,
                        dir_addr,
                    )
                    .await
                    .expect("failed to re-establish connection to directory");

                    if let Err(e) = connection.send_plain(&req).await {
                        error!("failed to send message: {}", e);
                        continue;
                    }

                    info!("registering with directory server");
                    let resp =
                        connection.receive_plain::<DirectoryResponse>().await;

                    if handle_response(resp, &mut timer, &duration)
                        .await
                        .is_err()
                    {
                        continue;
                    }
                }
            }
            .instrument(
                trace_span!("directory_renew", local=%local, server=%dir_addr),
            ),
        ))
    }

    /// Close this `Listener` and stops the renewing of the directory entry.
    pub async fn close(self) -> Result<(), ListenerError> {
        let _ = self.exit_tx.send(());
        Ok(())
    }
}

async fn check_connection(
    connector: &mut dyn Connector<Candidate = SocketAddr>,
    connection: &mut Connection,
    pkey: &PublicKey,
    dir_addr: SocketAddr,
) -> Result<(), ()> {
    if connection.is_broken() {
        error!("lost connection to directory, reconnecting");

        *connection = match connector.establish(&pkey, &dir_addr).await {
            Ok(socket) => Connection::new(socket),
            Err(e) => {
                error!("failed to reconnect to directory: {}", e);
                return Err(());
            }
        }
    }

    Ok(())
}

async fn handle_response(
    resp: Result<DirectoryResponse, ReceiveError>,
    timer: &mut Interval,
    duration: &Duration,
) -> Result<(), ()> {
    match resp {
        Ok(DirectoryResponse::Ok) => {
            info!(
                "renewed lease successfully, next renew in {} seconds",
                duration.as_secs(),
            );
            timer.tick().await;
            Ok(())
        }
        Ok(_) => {
            error!("invalid response from directory: {:?}", resp);
            Err(())
        }
        Err(e) => {
            error!("failed to renew registration: {}", e);
            Err(())
        }
    }
}

#[async_trait]
impl Listener for DirectoryListener {
    type Candidate = SocketAddr;

    async fn establish(&mut self) -> Result<Box<dyn Socket>, ListenerError> {
        Ok(self.listener.establish().await?)
    }

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr()
    }

    fn exchanger(&self) -> &Exchanger {
        self.listener.exchanger()
    }

    async fn candidates(&self) -> Result<&[Self::Candidate], ListenerError> {
        todo!()
    }
}

impl fmt::Display for DirectoryListener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "directory listener at {}", self.local_addr().unwrap(),)
    }
}

#[cfg(test)]
mod test {
    use super::super::super::connector::TcpDirect;
    use super::super::super::listener::TcpListener;
    use super::*;
    use crate::crypto::key::exchange::Exchanger;
    use crate::test::*;

    use tokio::task;

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

            let mut connection = Connection::new(
                listener.establish().await.expect("accept failed"),
            );

            let request = connection
                .receive_plain::<DirectoryRequest>()
                .await
                .expect("read request failed");

            assert_eq!(
                request,
                DirectoryRequest::Add((srv_pub, list_addr).into()),
                "bad request"
            );

            connection
                .send_plain(&DirectoryResponse::Ok)
                .await
                .expect("response failed");

            let mut connector = TcpDirect::new(Exchanger::random());

            connector
                .connect(&srv_pub, &list_addr)
                .await
                .expect("connect failed");
        });

        let connector = TcpDirect::new(server_exchanger);
        let mut listener = DirectoryListener::new(
            Box::new(dir_listener),
            Box::new(connector),
            dir_server,
        )
        .await
        .expect("dir_bind failed");

        listener.accept().await.expect("accept failed");

        handle.await.expect("dir server failed");
    }
}
