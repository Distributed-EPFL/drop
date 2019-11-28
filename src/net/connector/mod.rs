/// Tcp related connectors
pub mod tcp;

use self::tcp::TcpError;
use super::Connection;
use crate as drop;
use crate::crypto::stream::Push as KeyExchanger;
use crate::crypto::Key as PublicKey;
use crate::error::Error;

use async_trait::async_trait;

use bincode::ErrorKind as BincodeErrorKind;
use bincode::{deserialize, serialize};

use macros::error;

use serde::{Deserialize, Serialize};

use tokio::net::ToSocketAddrs;

pub type SerializerError = Box<BincodeErrorKind>;

error! {
    type: ReadError,
    description: "error reading",
    causes: (SerializerError, TcpError)
}

error! {
    type: WriteError,
    description: "error writing",
    causes: (SerializerError, TcpError),
}

/// The `Connector` trait is used to connect to peers using some sort of
/// Internet address (e.g. Ipv4 or Ipv6).
#[async_trait]
pub trait Connector {
    /// The target address type used by this connector
    type Addr: ToSocketAddrs;

    /// The concrete type of `Connection` that this `Connector` will produce
    type Connection;

    /// The type of error that this `Connector` will return
    type Error;

    /// Connect asynchronously to a given destination with its `PublicKey` and
    /// the local node's `KeyExchanger`
    async fn connect(
        addr: Self::Addr,
        exchanger: KeyExchanger,
        pkey: &PublicKey,
    ) -> Result<Self::Connection, Self::Error>;
}

/// A concrete struct used on top of `Connection` to avoid the trait object
/// can't have generic methods problem.
pub struct Channel<C: Connection<Error = E>, E>
where
    ReadError: From<E>,
    WriteError: From<E>,
{
    connection: C,
}

impl<C: Connection<Error = E>, E> Channel<C, E>
where
    ReadError: From<E> + Send,
    WriteError: From<E> + Send,
{
    /// Create a new `ReaderWriter` using a specified `Connection`
    pub fn new(connection: C) -> Self {
        Self { connection }
    }

    /// Receive a deserializiable message from the underlying connection
    pub async fn receive<T>(&mut self) -> Result<T, ReadError>
    where
        T: Sized + for<'de> Deserialize<'de> + Send,
    {
        let mut buf = [0u8; 256];

        let read = match self.connection.receive(&mut buf).await {
            Ok(v) => v,
            Err(e) => return Err(e.into()),
        };

        deserialize(&buf[..read]).map_err(|e| e.into())
    }

    /// Send a serializable message using the underlying connection
    pub async fn send<T>(&mut self, message: &T) -> Result<usize, WriteError>
    where
        T: Serialize,
    {
        let serialized = serialize(message)?;

        self.connection
            .send(&serialized)
            .await
            .map_err(|e| e.into())
    }
}
