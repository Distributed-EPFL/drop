/// Utilities to connect to other peers in a secure fashion
pub mod connector;

/// Utilities to open a listener for incoming connections
pub mod listener;

use std::fmt;
use std::io::Error as IoError;
use std::mem;
use std::net::SocketAddr;

use crate as drop;
use crate::crypto::key::exchange::{Exchanger, PublicKey};
use crate::crypto::stream::{Pull, Push};
use crate::crypto::{DecryptError, EncryptError, ExchangeError};
use crate::error::Error;

use bincode::{deserialize, serialize, ErrorKind as BincodeErrorKind};

use macros::error;

use serde::{Deserialize, Serialize};

use tokio::io::{
    AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, Error as TokioError,
};
use tokio::task::block_in_place;

/// Type of errors returned when serializing/deserializing
pub type SerializerError = Box<BincodeErrorKind>;

error! {
    type: SendError,
    description: "error sending data",
    causes: (EncryptError, SerializerError, CorruptedConnection,
             NeedsAuthError, IoError)
}

error! {
    type: ReceiveError,
    description: "error receiving data",
    causes: (DecryptError, SerializerError, CorruptedConnection,
             NeedsAuthError, IoError)
}

error! {
    type: SecureError,
    description: "error securing channel",
    causes: (ExchangeError, TokioError, ReceiveError, SendError)
}

error! {
    type: NeedsAuthError,
    description: "connection needs authentication",
}

error! {
    type: CorruptedConnection,
    description: "channel corrupted"
}

/// Trait for structs that are able to asynchronously send and receive data from
/// the network. Only requirement is asynchronously reading and writing arrays
/// of bytes
pub trait Socket: AsyncRead + AsyncWrite + Unpin + Send + Sync {
    /// Address of the remote peer for this `Connection`
    fn remote(&self) -> Result<SocketAddr, IoError>;

    /// Local address in use by this `Connection`
    fn local(&self) -> Result<SocketAddr, IoError>;
}

/// Encrypted connection state
enum ChannelState {
    /// Connection state before attempting authentication
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
    state: ChannelState,
    buffer: Vec<u8>,
}

impl Connection {
    /// Create a new `Connection` using a specified `Socket`
    pub fn new(socket: Box<dyn Socket>) -> Self {
        Self {
            socket,
            state: ChannelState::Connected,
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
        let mut buf = [0u8; 4096];

        let read = self.socket.read(&mut buf).await?;

        deserialize(&buf[..read]).map_err(|e| e.into())
    }

    /// Send a `Serialize` message on this `Connection` without using decryption
    ///
    /// # Example
    /// ```ignore
    /// let mut connection: Connection;
    /// let written = connection.send_plain(&0u32).await?;
    /// ```
    pub async fn send_plain<T>(
        &mut self,
        message: &T,
    ) -> Result<usize, SendError>
    where
        T: Serialize,
    {
        let serialized = serialize(message)?;

        self.socket.write(&serialized).await.map_err(|e| e.into())
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
        T: Sized + for<'de> Deserialize<'de> + Send,
    {
        match &mut self.state {
            ChannelState::Secured(ref mut pull, _) => {
                let sz = Self::read_size(&mut *self.socket).await? as usize;

                // FIXME: avoid trusting network input and run out of memory
                self.buffer.resize(sz, 0);

                let read =
                    self.socket.read_exact(&mut self.buffer[..sz]).await?;

                pull.decrypt_ref(&self.buffer[..read]).map_err(|e| {
                    self.state = ChannelState::Broken;
                    e.into()
                })
            }
            ChannelState::Connected => Err(NeedsAuthError::new().into()),
            ChannelState::Broken => Err(CorruptedConnection::new().into()),
        }
    }

    /// Send a `Serialize` message on this `Connection` synchronously.
    /// This call may block if the data can't be sent immediately.
    pub fn send_sync<T>(&mut self, _message: &T) -> Result<usize, SendError>
    where
        T: Serialize + Send,
    {
        block_in_place(|| unimplemented!())
    }

    /// Send a `Serialize` message using the underlying `Connection`.
    pub async fn send_async<T>(
        &mut self,
        message: &T,
    ) -> Result<usize, SendError>
    where
        T: Serialize + Send,
    {
        match &mut self.state {
            ChannelState::Secured(_, ref mut push) => {
                let data = push.encrypt(message)?;

                Self::write_size(&mut *self.socket, data.len() as u32).await?;

                self.socket.write(&data).await.map_err(|e| e.into())
            }
            ChannelState::Connected => Err(NeedsAuthError::new().into()),
            ChannelState::Broken => Err(CorruptedConnection::new().into()),
        }
    }

    /// Send a `Serialize` message in a synchronous fashion. This method may
    /// block
    pub async fn send<T>(&mut self, message: &T) -> Result<usize, SendError>
    where
        T: Serialize + Send,
    {
        self.send_async(message).await
    }

    /// Perform the key exchange and create a new `Session`
    fn exchange(
        &mut self,
        exchanger: &Exchanger,
        remote: &PublicKey,
    ) -> Result<(), SecureError> {
        let session = exchanger.exchange(remote)?;
        let (push, pull): (Push, Pull) = session.into();

        self.state = ChannelState::Secured(pull, push);

        Ok(())
    }

    /// Secures the `Connection` to a server
    pub async fn secure_server(
        &mut self,
        local: &Exchanger,
        server: &PublicKey,
    ) -> Result<(), SecureError> {
        self.send_plain(local.keypair().public()).await?;

        self.exchange(local, server)?;

        Ok(())
    }

    /// Secures this `Connection` from a client
    pub async fn secure_client(
        &mut self,
        exchanger: &Exchanger,
    ) -> Result<(), SecureError> {
        let pkey = self.receive_plain::<PublicKey>().await?;

        self.exchange(exchanger, &pkey)?;

        Ok(())
    }

    /// Gracefully closes this `Connection` ensuring that any data sent has been
    /// received by the remote peer.
    pub async fn close(&mut self) -> Result<(), IoError> {
        self.socket.shutdown().await
    }

    /// Checks whether this `Connection` is secured
    pub fn is_secured(&self) -> bool {
        match &self.state {
            ChannelState::Secured(_, _) => true,
            _ => false,
        }
    }

    /// Checks whether this `Connection` is in a usable state
    pub fn is_broken(&self) -> bool {
        match &self.state {
            ChannelState::Broken => true,
            _ => false,
        }
    }
}

impl fmt::Debug for Connection {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let (local, remote) = match (self.socket.local(), self.socket.remote())
        {
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

        write!(f, "{} channel {} -> {}", sec, local, remote)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::ToSocketAddrs;

    use super::*;

    use crate::crypto::key::exchange::{Exchanger, KeyPair};
    use crate::net::connector::tcp::TcpDirect;
    use crate::net::connector::Connector;
    use crate::net::listener::tcp::TcpListener;
    use crate::net::listener::Listener;

    use rand;

    use serde::{Deserialize, Serialize};

    pub const LISTENER_ADDR: &str = "localhost";

    /// Create two ends of a `Connection` using the specified `Listener`
    /// and `Connector` types
    macro_rules! generate_connection {
        ($listener:ty , $connector:ty) => {
            let client = KeyPair::random();
            let server = KeyPair::random();
            let client_ex = Exchanger::new(client.clone());
            let server_ex = Exchanger::new(server.clone());

            loop {
                let port: u16 = rand::random();
                let addr: SocketAddr = (LISTENER_ADDR, port)
                    .to_socket_addrs()
                    .expect("failed to parse localhost")
                    .as_slice()[0];
                let mut listener =
                    match <$listener>::new(addr, server_ex.clone()).await {
                        Ok(listener) => listener,
                        Err(_) => continue,
                    };

                let connector = <$connector>::new(client_ex);

                let outgoing = connector
                    .connect(server.public(), &addr)
                    .await
                    .expect("failed to connect");

                let incoming = listener
                    .accept()
                    .await
                    .expect("failed to accept incoming connection");

                assert!(
                    incoming.is_secured(),
                    "server coulnd't secure the connection"
                );

                assert!(
                    outgoing.is_secured(),
                    "client couldn't secure the connection"
                );

                return (outgoing, incoming);
            }
        };
    }

    /// Exchanges the given data using a new `Connection` and checks that the
    /// received data is the same as what was sent.
    macro_rules! exchange_data_and_compare {
        ($data:expr, $type:ty, $setup:ident) => {
            let (mut client, mut listener) = $setup().await;

            let data = $data;

            client.send(&data).await.expect("failed to send");

            let recvd: $type =
                listener.receive().await.expect("failed to receive");

            assert_eq!(data, recvd, "data is not the same");
        };
    }

    pub async fn setup_tcp() -> (Connection, Connection) {
        generate_connection!(TcpListener, TcpDirect);
    }

    #[tokio::test]
    async fn tcp_u8_exchange() {
        exchange_data_and_compare!(0, u8, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_u16_exchange() {
        exchange_data_and_compare!(0, u16, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_u32_exchange() {
        exchange_data_and_compare!(0, u32, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_u64_exchange() {
        exchange_data_and_compare!(0, u64, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_i8_exchange() {
        exchange_data_and_compare!(0, i8, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_i16_exchange() {
        exchange_data_and_compare!(0, i16, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_i32_exchange() {
        exchange_data_and_compare!(0, i32, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_i64_exchange() {
        exchange_data_and_compare!(0, i64, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_struct_exchange() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct T {
            a: u32,
            b: u64,
            c: A,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct A {
            a: u8,
            b: u16,
        }

        let data = T {
            a: 258,
            b: 30567,
            c: A { a: 66, b: 245 },
        };

        exchange_data_and_compare!(data, T, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_hashmap_exchange() {
        let mut hashmap: HashMap<u32, u128> = HashMap::default();

        for _ in 0..rand::random::<usize>() % 2048 {
            hashmap.insert(rand::random(), rand::random());
        }

        exchange_data_and_compare!(hashmap, HashMap<u32, u128>, setup_tcp);
    }

    #[tokio::test]
    async fn garbage_data_decryption() {
        let (mut client, mut listener) = setup_tcp().await;

        client
            .send_plain(&0u32)
            .await
            .expect("failed to send unencrypted data");

        listener
            .receive::<u32>()
            .await
            .expect_err("received garbage correctly");

        assert!(
            listener.is_broken(),
            "incorrect state for listener connection"
        );
    }

    #[tokio::test]
    async fn initial_state() {
        let (client, listener) = setup_tcp().await;

        assert!(client.is_secured(), "client is not authenticated");
        assert!(listener.is_secured(), "listener is not authenticated");
        assert!(!listener.is_broken(), "listener is errored");
        assert!(!client.is_broken(), "client is errored");
    }

    #[tokio::test]
    async fn connection_fmt() {
        let (client, _listener) = setup_tcp().await;

        assert_eq!(
            format!("{:?}", client),
            format!(
                "secure channel {} -> {}",
                client.socket.local().unwrap(),
                client.socket.remote().unwrap()
            )
        );
    }

    #[tokio::test]
    async fn tcp_non_existent() {
        let exchanger = Exchanger::random();
        let keypair = KeyPair::random();
        let connector = TcpDirect::new(exchanger);
        let port: u16 = rand::random();
        let addr =
            (LISTENER_ADDR, port).to_socket_addrs().unwrap().as_slice()[0];

        connector
            .connect(keypair.public(), &addr)
            .await
            .expect_err("connected to non-existent listener");
    }
}
