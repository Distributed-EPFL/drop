use crate::{
    crypto::{
        key::exchange::{Exchanger, PublicKey},
        stream::{Pull, Push},
    },
    net::{
        connection::errors::{
            CorruptedReceive, CorruptedSend, Decrypt, DeserializeReceive,
            Encrypt, Exchange, ReceiveError, ReceiveIo, SecureError,
            SecureReceive, SecureSend, SendError, SendIo, SerializeSend,
            UnsecuredReceive, UnsecuredSend,
        },
        socket::Socket,
    },
};

use bincode;

use serde::{Deserialize, Serialize};

use snafu::ResultExt;

use std::net::SocketAddr;
use std::{fmt, io, mem};

use tokio::io::{
    split, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadHalf,
    WriteHalf,
};

use tracing::{debug, debug_span, info};
use tracing_futures::Instrument;

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
    remote_pkey: Option<PublicKey>,
}

impl Connection {
    /// Create a new `Connection` using a specified `Socket`
    pub fn new(socket: Box<dyn Socket>) -> Self {
        Self {
            socket,
            state: ConnectionState::Connected,
            buffer: Vec::new(),
            remote_pkey: None,
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
        let size = Self::read_size(&mut self.socket).await? as usize;

        self.buffer.resize(size, 0);

        self.socket
            .read_exact(&mut self.buffer[..size])
            .await
            .map_err(|e| {
                self.state = ConnectionState::Broken;
                e
            })
            .context(ReceiveIo)?;

        bincode::deserialize(&self.buffer)
            .context(DeserializeReceive)
            .map_err(|e| {
                self.state = ConnectionState::Broken;
                e
            })
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
        let serialized = bincode::serialize(message).context(SerializeSend)?;

        debug!("sending {} bytes as plain data", serialized.len());

        Self::write_size(&mut self.socket, serialized.len() as u32)
            .await
            .map_err(|e| {
                self.state = ConnectionState::Broken;
                e
            })?;

        self.socket
            .write_all(&serialized)
            .await
            .map_err(|e| {
                self.state = ConnectionState::Broken;
                e
            })
            .context(SendIo)
    }

    async fn read_size<R: AsyncRead + Unpin + ?Sized>(
        socket: &mut R,
    ) -> Result<u32, ReceiveError> {
        let mut buf = [0u8; mem::size_of::<u32>()];
        socket.read_exact(&mut buf).await.context(ReceiveIo)?;

        bincode::deserialize(&buf[..]).context(DeserializeReceive)
    }

    async fn write_size<W: AsyncWrite + Unpin>(
        socket: &mut W,
        size: u32,
    ) -> Result<(), SendError> {
        let data = bincode::serialize(&size).context(SerializeSend)?;

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
                Self::receive_internal(
                    pull,
                    self.socket.as_mut(),
                    &mut self.buffer,
                )
                .await
                .map_err(|e| {
                    self.state = ConnectionState::Broken;
                    e
                })
            }
            ConnectionState::Connected => UnsecuredReceive.fail(),
            ConnectionState::Broken => CorruptedReceive.fail(),
        }
    }

    async fn receive_internal<
        T: Sized + for<'de> Deserialize<'de> + Send + fmt::Debug,
        R: AsyncRead + Unpin + ?Sized,
    >(
        pull: &mut Pull,
        socket: &mut R,
        mut buffer: &mut Vec<u8>,
    ) -> Result<T, ReceiveError> {
        let size = Connection::read_size(socket)
            .instrument(debug_span!("read_size"))
            .await? as usize;

        // FIXME: avoid trusting network input and run out of memory
        buffer.resize(size, 0);

        socket
            .read_exact(&mut buffer)
            .instrument(debug_span!("read_data"))
            .await
            .context(ReceiveIo)?;

        pull.decrypt(buffer).context(Decrypt)
    }

    /// Send a `Serialize` message using the underlying `Connection`.
    pub async fn send<T>(&mut self, message: &T) -> Result<(), SendError>
    where
        T: Serialize + Send + fmt::Debug,
    {
        match &mut self.state {
            ConnectionState::Secured(_, ref mut push) => {
                Self::send_internal(message, &mut self.socket, push)
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

    async fn send_internal<
        T: Serialize + Send + fmt::Debug,
        W: AsyncWrite + Unpin,
    >(
        message: &T,
        socket: &mut W,
        push: &mut Push,
    ) -> Result<(), SendError> {
        let data = push.encrypt(message).context(Encrypt)?;

        Connection::write_size(socket, data.len() as u32).await?;

        socket.write_all(&data).await.context(SendIo)
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

    /// Returns the remote end's `PublicKey`. Returns `None` if key exchange
    /// has not been performed on this `Connection`
    pub fn remote_key(&self) -> Option<PublicKey> {
        self.remote_pkey
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

        self.remote_pkey = Some(*server);

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

        self.remote_pkey = Some(pkey);

        Ok(())
    }

    /// Gracefully closes this `Connection` ensuring that any data sent has been
    /// received by the remote peer.
    pub async fn close(&mut self) -> Result<(), io::Error> {
        self.socket.shutdown().await
    }

    /// Flushes any pending data waiting to be received by the other end of this
    /// `Connection` only returning when all data has been acknowledged by the
    /// remote peer.
    pub async fn flush(&mut self) -> Result<(), io::Error> {
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
    pub fn peer_addr(&self) -> Result<SocketAddr, io::Error> {
        self.socket.peer_addr()
    }

    /// Get the local address of this `Connection`
    pub fn local_addr(&self) -> Result<SocketAddr, io::Error> {
        self.socket.local_addr()
    }

    /// Split this `Connection` into a `ReadHalf` and a `WriteHalf` to allow
    /// simultaneous reading and writing from the same `Connection`.
    /// This returns `None` if the `Connection` wasn't secured prior to this
    /// call.
    pub fn split(self) -> Option<(ConnectionRead, ConnectionWrite)> {
        match self.state {
            ConnectionState::Secured(pull, push) => {
                let (read, write) = split(self.socket);
                let writer = ConnectionWrite {
                    write,
                    push,
                    remote: self.remote_pkey.unwrap(),
                };
                let reader = ConnectionRead {
                    read,
                    pull,
                    buffer: Vec::with_capacity(4096),
                    remote: self.remote_pkey.unwrap(),
                };

                Some((reader, writer))
            }
            _ => None,
        }
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

/// The read end of a `Connection` resulting from `Connection::split`
pub struct ConnectionRead {
    read: ReadHalf<Box<dyn Socket>>,
    pull: Pull,
    remote: PublicKey,
    buffer: Vec<u8>,
}

impl ConnectionRead {
    /// See `Connection::receive`for more details
    pub async fn receive<T: for<'de> Deserialize<'de> + fmt::Debug + Send>(
        &mut self,
    ) -> Result<T, ReceiveError> {
        Connection::receive_internal(
            &mut self.pull,
            &mut self.read,
            &mut self.buffer,
        )
        .await
    }

    /// Get the `PublicKey` associated with this `ConnectionRead`
    pub fn remote_pkey(&self) -> &PublicKey {
        &self.remote
    }
}

impl fmt::Display for ConnectionRead {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "connection read end for {}", self.remote)
    }
}

/// The write end of `Connection` resulting from `Connection::split`
pub struct ConnectionWrite {
    write: WriteHalf<Box<dyn Socket>>,
    push: Push,
    remote: PublicKey,
}

impl ConnectionWrite {
    /// See `Connection::send` for more details
    pub async fn send<M: Serialize + fmt::Debug + Send>(
        &mut self,
        message: &M,
    ) -> Result<(), SendError> {
        Connection::send_internal(message, &mut self.write, &mut self.push)
            .await
    }

    /// Get the remote `PublicKey` associated with this `ConnectionWrite`
    pub fn remote_pkey(&self) -> &PublicKey {
        &self.remote
    }
}

impl fmt::Display for ConnectionWrite {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "connection write end for {}", self.remote)
    }
}
