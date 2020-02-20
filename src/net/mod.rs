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
use crate::crypto::key::exchange::{ExchangeError, Exchanger, PublicKey};
use crate::crypto::stream::{DecryptError, EncryptError};
use crate::crypto::stream::{Pull, Push};

use bincode::{deserialize, serialize, ErrorKind as BincodeErrorKind};

use serde::{Deserialize, Serialize};

use snafu::{Backtrace, ResultExt, Snafu};

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use tracing::{debug, debug_span, info, trace};
use tracing_futures::Instrument;

/// Type of errors returned when serializing/deserializing
pub type SerializerError = Box<BincodeErrorKind>;

#[derive(Debug, Snafu)]
/// Error encountered when attempting to send data on a `Connection`
pub enum SendError {
    #[snafu(display("could not encrypt data: {}", source))]
    /// Error encrypting data before sending
    Encrypt { source: EncryptError },

    #[snafu(display("could not serialize data: {}", source))]
    /// Data could not be serialized for sending
    SerializeSend { source: SerializerError },

    #[snafu(display("i/o error: {}", source))]
    /// OS error encountered when sending
    SendIo { source: IoError },

    #[snafu(display("connection corrupted"))]
    /// Attempted to send data on a corrupted `Connection`
    CorruptedSend { backtrace: Backtrace },

    #[snafu(display("unsecured connection"))]
    /// Attempted to send data on an unsecured `Connection`
    UnsecuredSend { backtrace: Backtrace },
}

#[derive(Debug, Snafu)]
/// Error encountered when attempting to receive data on a `Connection`
pub enum ReceiveError {
    #[snafu(display("could not decrypt data: {}", source))]
    /// Error decrypting received data
    Decrypt { source: DecryptError },

    #[snafu(display("connection is corrupted"))]
    /// Attempted to read from a corrupted `Connection`
    CorruptedReceive { backtrace: Backtrace },

    #[snafu(display("deserialization error: {}", source))]
    /// Error deserializing received data
    DeserializeReceive { source: SerializerError },

    #[snafu(display("unsecured connection"))]
    /// Attempting a secure receive on an unsecured `Connection`
    UnsecuredReceive { backtrace: Backtrace },

    #[snafu(display("i/o error: {}", source))]
    /// OS error encountered
    ReceiveIo { source: IoError },
}

#[derive(Debug, Snafu)]
/// Error encountered when attempting to secure a `Connection`
pub enum SecureError {
    #[snafu(display("could not exchange keys: {}", source))]
    /// Keys could not be exchanged properly
    Exchange { source: ExchangeError },

    #[snafu(display("i/o error: {}", source))]
    /// OS error occurred while handshaking
    SecureIo { source: IoError },

    #[snafu(display("receive error: {}", source))]
    /// Error receiving data during handshake
    SecureReceive { source: ReceiveError },

    #[snafu(display("send error :{}", source))]
    /// Error sending data during handshake
    SecureSend { source: SendError },
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

        self.socket
            .read_exact(&mut self.buffer[..size])
            .await
            .context(ReceiveIo)?;

        deserialize(&self.buffer).context(DeserializeReceive)
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
        let serialized = serialize(message).context(SerializeSend)?;

        debug!("sending {} bytes as plain data", serialized.len());

        Self::write_size(self.socket.as_mut(), serialized.len() as u32).await?;

        self.socket.write_all(&serialized).await.context(SendIo)
    }

    async fn read_size(socket: &mut dyn Socket) -> Result<u32, ReceiveError> {
        let mut buf = [0u8; mem::size_of::<u32>()];
        socket.read_exact(&mut buf).await.context(ReceiveIo)?;

        deserialize(&buf[..]).context(DeserializeReceive)
    }

    async fn write_size(
        socket: &mut dyn Socket,
        size: u32,
    ) -> Result<(), SendError> {
        let data = serialize(&size).context(SerializeSend)?;

        socket.write_all(&data).await.context(SendIo)
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
                        socket.peer_addr().context(ReceiveIo)?
                    );

                    // FIXME: avoid trusting network input and run out of memory
                    buffer.resize(size, 0);

                    socket
                        .read_exact(&mut buffer)
                        .instrument(debug_span!("read_data"))
                        .await
                        .context(ReceiveIo)?;

                    pull.decrypt(buffer).context(Decrypt)
                }

                receive_internal(pull, self.socket.as_mut(), &mut self.buffer)
                    .await
            }
            ConnectionState::Connected => UnsecuredReceive.fail(),
            ConnectionState::Broken => CorruptedReceive.fail(),
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
                    let data = push.encrypt(message).context(Encrypt)?;

                    trace!(
                        "sending {} bytes of data to {}",
                        data.len(),
                        socket.peer_addr().context(SendIo)?
                    );

                    Connection::write_size(socket, data.len() as u32).await?;

                    socket.write_all(&data).await.context(SendIo)
                }

                send_internal(message, self.socket.as_mut(), push)
                    .await
                    .map_err(|e| {
                        self.state = ConnectionState::Broken;
                        e
                    })
            }
            ConnectionState::Connected => UnsecuredSend.fail(),
            ConnectionState::Broken => CorruptedSend.fail(),
        }
    }

    /// Perform the key exchange and create a new `Session`
    fn exchange(
        &mut self,
        exchanger: &Exchanger,
        remote: &PublicKey,
    ) -> Result<(), SecureError> {
        let session = exchanger.exchange(remote).context(Exchange)?;
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
        self.send_plain(local.keypair().public())
            .await
            .context(SecureSend)?;

        self.exchange(local, server)?;

        Ok(())
    }

    /// Secures this `Connection` from a client
    pub async fn secure_client(
        &mut self,
        exchanger: &Exchanger,
    ) -> Result<(), SecureError> {
        info!("waiting for peer's public key");
        let pkey = self
            .receive_plain::<PublicKey>()
            .await
            .context(SecureReceive)?;

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
        matches!(&self.state, ConnectionState::Secured(_, _))
    }

    /// Checks whether this `Connection` is in a usable state
    pub fn is_broken(&self) -> bool {
        matches!(&self.state, ConnectionState::Broken)
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
