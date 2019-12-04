/// Utilities to connect to other peers in a secure fashion
pub mod connector;

/// Utilities to open a listener for incoming connections
pub mod listener;

use std::fmt;
use std::io::Error as IoError;
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

/// Type of errors returned when serializing/deserializing
pub type SerializerError = Box<BincodeErrorKind>;

error! {
    type: SendError,
    description: "error sending data",
    causes: (EncryptError, SocketError, SerializerError, CorruptedConnection,
             NeedsAuthError, IoError)
}

error! {
    type: ReceiveError,
    description: "error receiving data",
    causes: (DecryptError, SocketError, SerializerError, CorruptedConnection,
             NeedsAuthError, IoError)
}

error! {
    type: SecureError,
    description: "error securing channel",
    causes: (ExchangeError, TokioError, ReceiveError, SendError)
}

error! {
    type: SocketError,
    description: "socket error",
    causes: (TokioError, ExchangeError, IoError)
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
pub trait Socket: AsyncRead + AsyncWrite + Unpin {
    /// Address of the remote peer for this `Connection`
    fn remote(&self) -> Result<SocketAddr, SocketError>;

    /// Local address in use by this `Connection`
    fn local(&self) -> Result<SocketAddr, SocketError>;
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
    exchanger: Exchanger,
    state: ChannelState,
}

impl Connection {
    /// Create a new `Connection` using a specified `Socket`
    pub fn new(socket: Box<dyn Socket>) -> Self {
        Self {
            socket,
            state: ChannelState::Connected,
        }
    }

    /// Receive `Deserialize` message on this `Connection` without using
    /// encryption
    pub async fn receive_plain<T>(&mut self) -> Result<T, ReceiveError>
    where
        T: for<'de> Deserialize<'de> + Sized,
    {
        let mut buf = [0u8; 4096];

        let read = self.socket.read(&mut buf).await?;

        deserialize(&buf[..read]).map_err(|e| e.into())
    }

    /// Send a `Serialize` message on this `Connection` without using decryption
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

    /// Receive a `Deserialize` message from the underlying `Connection`
    pub async fn receive<T>(&mut self) -> Result<T, ReceiveError>
    where
        T: Sized + for<'de> Deserialize<'de> + Send,
    {
        match &mut self.state {
            ChannelState::Secured(ref mut pull, _) => {
                let mut buf = [0u8; 4096]; // FIXME: some saner size for the buffer

                let read = self.socket.read(&mut buf).await?;

                pull.decrypt_ref(&buf[..read]).map_err(|e| {
                    self.state = ChannelState::Broken;
                    e.into()
                })
            }
            ChannelState::Connected => Err(NeedsAuthError::new().into()),
            ChannelState::Broken => Err(CorruptedConnection::new().into()),
        }
    }

    /// Send a `Serialize` message using the underlying `Connection`.
    pub async fn send<T>(&mut self, message: &T) -> Result<usize, SendError>
    where
        T: Serialize + Send,
    {
        match &mut self.state {
            ChannelState::Secured(_, ref mut push) => {
                let data = push.encrypt(message)?;

                self.socket.write(&data).await.map_err(|e| e.into())
            }
            ChannelState::Connected => Err(NeedsAuthError::new().into()),
            ChannelState::Broken => Err(CorruptedConnection::new().into()),
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

    /// Checks whether this `Connection` is secured
    pub fn is_secured(&self) -> bool {
        match &self.state {
            ChannelState::Secured(_, _) => true,
            _ => false,
        }
    }

    /// Checks whether this `Channel` is in a usable state
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

        write!(f, "secure channel {} -> {}", local, remote)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    use crate::crypto::key::exchange::{Exchanger, KeyPair};
    use crate::net::connector::tcp::TcpDirect;
    use crate::net::connector::Connector;
    use crate::net::listener::tcp::TcpListener;
    use crate::net::listener::Listener;

    use rand;

    use serde::{Deserialize, Serialize};

    pub const LISTENER_ADDR: &str = "127.0.0.1:9999";

    macro_rules! exchange_data_and_compare {
        ($data:expr) => {
            let (mut client, mut listener) = setup_listener_and_client().await;

            let data = $data;

            client.send(&data).await.expect("failed to send");

            let recvd = listener.receive().await.expect("failed to receive");

            assert_eq!(data, recvd, "data is not the same");
        };
    }

    async fn setup_listener_and_client() -> (Connection, Connection) {
        let client = KeyPair::random();
        let server = KeyPair::random();
        let client_ex = Exchanger::new(client.clone());
        let server_ex = Exchanger::new(server.clone());

        let mut listener = TcpListener::new(LISTENER_ADDR, server_ex)
            .await
            .expect("failed to bind");

        let connector = TcpDirect::new(client_ex);

        let client = connector
            .connect(server.public(), LISTENER_ADDR.parse().unwrap())
            .await
            .expect("failed to connect");

        let listener = listener
            .accept()
            .await
            .expect("failed to accept incoming connection");

        assert!(
            listener.is_secured(),
            "server coulnd't secure the connection"
        );
        assert!(client.is_secured(), "client couldn't secure the connection");

        (client, listener)
    }

    #[tokio::test]
    async fn tcp_data_exchange() {
        let (mut client, mut listener) = setup_listener_and_client().await;

        client.send(&0u32).await.expect("failed to send data");

        let recvd = listener.receive::<u32>().await.expect("failed to receive");

        assert_eq!(0u32, recvd, "data received incorrect");
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

        exchange_data_and_compare!(data);
    }

    #[tokio::test]
    async fn tcp_hashmap_exchange() {
        let mut hashmap: HashMap<u32, u128> = HashMap::default();

        for _ in 0..rand::random::<usize>() % 2048 {
            hashmap.insert(rand::random(), rand::random());
        }

        exchange_data_and_compare!(hashmap);
    }

    #[tokio::test]
    async fn garbage_data_decryption() {
        let (mut client, mut listener) = setup_listener_and_client().await;

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
        let (client, listener) = setup_listener_and_client().await;

        assert!(client.is_secured(), "client is not authenticated");
        assert!(listener.is_secured(), "listener is not authenticated");
        assert!(!listener.is_broken(), "listener is errored");
        assert!(!client.is_broken(), "client is errored");
    }

    #[tokio::test]
    async fn tcp_non_existent() {
        let exchanger = Exchanger::random();
        let keypair = KeyPair::random();
        let connector = TcpDirect::new(exchanger);

        connector
            .connect(keypair.public(), LISTENER_ADDR.parse().unwrap())
            .await
            .expect_err("connected to non-existent listener");
    }
}
