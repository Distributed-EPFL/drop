/// Common data shared between `Listener`s and `Connector`s
pub(crate) mod common;

/// Utilities to connect to other peers in a secure fashion
pub mod connector;
pub use connector::ConnectError;
pub use connector::Connector;
pub use connector::Directory as DirectoryConnector;
pub use connector::Tcp as TcpConnector;
pub use connector::Utp as UtpConnector;

/// Utilities to accept incoming connections from peers
pub mod listener;
pub use listener::Directory as DirectoryListener;
pub use listener::Listener;
pub use listener::ListenerError;
pub use listener::Tcp as TcpListener;
pub use listener::Utp as UtpListener;

/// Socket implementation for various types
mod socket;

/// Pre-made servers that accomplish common tasks
pub mod server;
pub use server::DirectoryServer;
pub use server::ServerError;

pub(self) mod utils;

use std::fmt;
use std::io::Error as IoError;
use std::mem;
use std::net::SocketAddr;

use self::socket::Socket;
use crate as drop;
use crate::crypto::key::exchange::{Exchanger, PublicKey};
use crate::crypto::stream::{Pull, Push};
use crate::crypto::{DecryptError, EncryptError, ExchangeError};
use crate::error::Error;

use bincode::{deserialize, serialize, ErrorKind as BincodeErrorKind};

use macros::error;

use serde::{Deserialize, Serialize};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use tracing::{debug, debug_span, info, trace};
use tracing_futures::Instrument;

/// Type of errors returned when serializing/deserializing
pub type SerializerError = Box<BincodeErrorKind>;

error! {
    type: SendError,
    description: "error sending data",
    causes: (EncryptError, SerializerError, CorruptedConnection,
             NeedsSecureError, IoError)
}

error! {
    type: ReceiveError,
    description: "error receiving data",
    causes: (DecryptError, SerializerError, CorruptedConnection,
             NeedsSecureError, IoError)
}

error! {
    type: SecureError,
    description: "error securing channel",
    causes: (ExchangeError, IoError, ReceiveError, SendError)
}

error! {
    type: NeedsSecureError,
    description: "connection needs to be secured",
}

error! {
    type: CorruptedConnection,
    description: "channel corrupted"
}

/// Encrypted connection state
enum ConnectionState {
    /// Connection state before exchanging keys
    Connected,
    /// Connection state once authentication has succeeded
    Secured(Pull, Push),
    /// Connection state after some error has been encountered
    Broken,
}

/// A `Connection` is a two way encrypted and authenticated communication
/// channel between two peers.
pub struct Connection {
    socket: Box<dyn Socket>,
    state: ConnectionState,
    buffer: Vec<u8>,
}

impl Connection {
    /// Create a new `Connection` using a specified `Socket`
    pub fn new(socket: Box<dyn Socket>) -> Self {
        Self {
            socket,
            state: ConnectionState::Connected,
            buffer: Vec::new(),
        }
    }

    /// Receive `Deserialize` message on this `Connection` without using
    /// encryption
    ///
    /// # Example
    /// ```ignore
    /// let mut connection;
    /// let value: u32 = connection.receive_plain().await?;
    /// ```
    pub async fn receive_plain<T>(&mut self) -> Result<T, ReceiveError>
    where
        T: for<'de> Deserialize<'de> + Sized,
    {
        let size = Self::read_size(self.socket.as_mut()).await? as usize;

        self.buffer.resize(size, 0);

        self.socket.read_exact(&mut self.buffer[..size]).await?;

        deserialize(&self.buffer).map_err(|e| e.into())
    }

    /// Send a `Serialize` message on this `Connection` without using decryption
    ///
    /// # Example
    /// ```ignore
    /// let mut connection: Connection;
    /// let written = connection.send_plain(&0u32).await?;
    /// ```
    pub async fn send_plain<T>(&mut self, message: &T) -> Result<(), SendError>
    where
        T: Serialize,
    {
        let serialized = serialize(message)?;

        debug!("sending {} bytes as plain data", serialized.len());

        Self::write_size(self.socket.as_mut(), serialized.len() as u32).await?;

        self.socket
            .write_all(&serialized)
            .await
            .map_err(|e| e.into())
    }

    async fn read_size(socket: &mut dyn Socket) -> Result<u32, ReceiveError> {
        let mut buf = [0u8; mem::size_of::<u32>()];
        socket.read_exact(&mut buf).await?;

        deserialize(&buf[..]).map_err(|e| e.into())
    }

    async fn write_size(
        socket: &mut dyn Socket,
        size: u32,
    ) -> Result<(), SendError> {
        let data = serialize(&size)?;

        socket.write_all(&data).await.map_err(|e| e.into())
    }

    /// Receive a `Deserialize` message from the underlying `Connection`.
    /// This will return an error if the `Connection` has not performed the
    /// key exchange prior to calling this method.
    pub async fn receive<T>(&mut self) -> Result<T, ReceiveError>
    where
        T: Sized + for<'de> Deserialize<'de> + Send + fmt::Debug,
    {
        match &mut self.state {
            ConnectionState::Secured(ref mut pull, _) => {
                async fn receive_internal<
                    T: Sized + for<'de> Deserialize<'de> + Send + fmt::Debug,
                >(
                    pull: &mut Pull,
                    socket: &mut dyn Socket,
                    mut buffer: &mut Vec<u8>,
                ) -> Result<T, ReceiveError> {
                    let size = Connection::read_size(socket)
                        .instrument(debug_span!("read_size"))
                        .await? as usize;

                    trace!(
                        "receiving message of {} bytes from {}",
                        size,
                        socket.peer_addr()?
                    );

                    // FIXME: avoid trusting network input and run out of memory
                    buffer.resize(size, 0);

                    socket
                        .read_exact(&mut buffer)
                        .instrument(debug_span!("read_data"))
                        .await?;

                    pull.decrypt(&buffer).map_err(|e| e.into())
                }

                let msg = receive_internal(
                    pull,
                    self.socket.as_mut(),
                    &mut self.buffer,
                )
                .await;

                if let Ok(ref msg) = msg {
                    debug!("received {:?}", msg);
                } else {
                    self.state = ConnectionState::Broken;
                }

                msg
            }
            ConnectionState::Connected => Err(NeedsSecureError::new().into()),
            ConnectionState::Broken => Err(CorruptedConnection::new().into()),
        }
    }

    /// Send a `Serialize` message using the underlying `Connection`.
    pub async fn send<T>(&mut self, message: &T) -> Result<(), SendError>
    where
        T: Serialize + Send + fmt::Debug,
    {
        match &mut self.state {
            ConnectionState::Secured(_, ref mut push) => {
                async fn send_internal<T: Serialize + Send + fmt::Debug>(
                    message: &T,
                    socket: &mut dyn Socket,
                    push: &mut Push,
                ) -> Result<(), SendError> {
                    let data = push.encrypt(message)?;

                    debug!("sending {:?}", message);

                    trace!(
                        "sending {} bytes of data to {}",
                        data.len(),
                        socket.peer_addr()?
                    );

                    Connection::write_size(socket, data.len() as u32).await?;

                    socket.write_all(&data).await.map_err(|e| e.into())
                }

                send_internal(message, self.socket.as_mut(), push)
                    .await
                    .map_err(|e| {
                        self.state = ConnectionState::Broken;
                        e
                    })
            }
            ConnectionState::Connected => Err(NeedsSecureError::new().into()),
            ConnectionState::Broken => Err(CorruptedConnection::new().into()),
        }
    }

    /// Perform the key exchange and create a new `Session`
    fn exchange(
        &mut self,
        exchanger: &Exchanger,
        remote: &PublicKey,
    ) -> Result<(), SecureError> {
        let session = exchanger.exchange(remote)?;
        let (push, pull): (Push, Pull) = session.into();

        self.state = ConnectionState::Secured(pull, push);

        Ok(())
    }

    /// Secures the `Connection` to a server
    pub async fn secure_server(
        &mut self,
        local: &Exchanger,
        server: &PublicKey,
    ) -> Result<(), SecureError> {
        info!("sending public key to peer");
        self.send_plain(local.keypair().public()).await?;

        self.exchange(local, server)?;

        Ok(())
    }

    /// Secures this `Connection` from a client
    pub async fn secure_client(
        &mut self,
        exchanger: &Exchanger,
    ) -> Result<(), SecureError> {
        info!("waiting for peer's public key");
        let pkey = self.receive_plain::<PublicKey>().await?;

        self.exchange(exchanger, &pkey)?;

        Ok(())
    }

    /// Gracefully closes this `Connection` ensuring that any data sent has been
    /// received by the remote peer.
    pub async fn close(&mut self) -> Result<(), IoError> {
        self.socket.shutdown().await
    }

    /// Flushes any pending data waiting to be received by the other end of this
    /// `Connection` only returning when all data has been acknowledged by the
    /// remote peer.
    pub async fn flush(&mut self) -> Result<(), IoError> {
        self.socket.flush().await
    }

    /// Checks whether this `Connection` is secured
    pub fn is_secured(&self) -> bool {
        match &self.state {
            ConnectionState::Secured(_, _) => true,
            _ => false,
        }
    }

    /// Checks whether this `Connection` is in a usable state
    pub fn is_broken(&self) -> bool {
        match &self.state {
            ConnectionState::Broken => true,
            _ => false,
        }
    }

    /// Get the address of the remote peer associated with this `Connection`
    pub fn peer_addr(&self) -> Result<SocketAddr, IoError> {
        self.socket.peer_addr()
    }

    /// Get the local address of this `Connection`
    pub fn local_addr(&self) -> Result<SocketAddr, IoError> {
        self.socket.local_addr()
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (local, remote) = match (self.local_addr(), self.peer_addr()) {
            (Ok(local), Ok(remote)) => {
                (format!("{}", local), format!("{}", remote))
            }
            (Ok(local), Err(_)) => {
                (format!("{}", local), "unknown".to_string())
            }
            (Err(_), Ok(remote)) => {
                ("unknown".to_string(), format!("{}", remote))
            }
            _ => ("unknown".to_string(), "unknown".to_string()),
        };

        let sec = if self.is_secured() {
            "secure"
        } else {
            "insecure"
        };

        write!(f, "{} connection {} -> {}", sec, local, remote)
    }
}
