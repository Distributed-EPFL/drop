/// Utilities to connect to other peers in a secure fashion
pub mod connector;

/// Utilities to open a listener for incoming connections
pub mod listener;

use std::io::Error as IoError;

use crate as drop;
use crate::crypto::key::exchange::{Exchanger, PublicKey};
use crate::crypto::stream::{Pull, Push};
use crate::crypto::{DecryptError, EncryptError, ExchangeError};
use crate::error::Error;

use macros::error;

use serde::{Deserialize, Serialize};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

error! {
    type: ChannelError,
    description: "channel error",
    causes: (ExchangeError, EncryptError, DecryptError, IoError, NeedsAuthError, CorruptedChannel)
}

error! {
    type: NeedsAuthError,
    description: "channel needs authentication",
}

error! {
    type: CorruptedChannel,
    description: "channel corrupted"
}

/// Trait for structs that are able to asynchronously send and receive data from
/// the network. Only requirement is asynchronously reading and writing arrays
/// of bytes
pub trait Connection: AsyncRead + AsyncWrite + Unpin {}

/// Encrypted channel state
enum ChannelState {
    /// Server channel before authentication
    ServerPreAuth,
    /// Client channel before authentication
    ClientPreAuth(PublicKey),
    /// Channel once authentication has succeeded
    Authenticated(Pull, Push),
    /// Channel state after some error has been encountered
    Errored,
}

/// A wrapper struct used on top of `Connection` to factor authentication
/// and serialization of data structures as well as avoiding the trait
/// object with generic parameters problem.
pub struct Channel<C: Connection> {
    connection: C,
    exchanger: Exchanger,
    state: ChannelState,
}

impl<C: Connection> Channel<C> {
    /// Create a new `ReaderWriter` using a specified `Connection`
    pub fn new_client(
        connection: C,
        exchanger: Exchanger,
        public: PublicKey,
    ) -> Self {
        Self {
            connection,
            exchanger,
            state: ChannelState::ClientPreAuth(public),
        }
    }

    /// Create a new server-end `Channel` using the given connection and key
    /// exchanger
    pub fn new_server(connection: C, exchanger: Exchanger) -> Self {
        Self {
            connection,
            exchanger,
            state: ChannelState::ServerPreAuth,
        }
    }

    /// Receive `Deserialize` message on this `Channel` without using encryption
    pub async fn receive_plain<T>(&mut self) -> Result<T, ChannelError> {
        unimplemented!()
    }

    /// Send a `Serialize` message on this `Channel` without using decryption
    pub async fn send_plain<T>(
        &mut self,
        message: T,
    ) -> Result<T, ChannelError> {
        unimplemented!()
    }

    /// Receive a `Deserialize` message from the underlying `Connection`
    pub async fn receive<T>(&mut self) -> Result<T, ChannelError>
    where
        T: Sized + for<'de> Deserialize<'de> + Send,
    {
        match &mut self.state {
            ChannelState::Authenticated(ref mut pull, _) => {
                let mut buf = [0u8; 256]; // FIXME: some saner size for the buffer

                let read = self.connection.read(&mut buf).await?;

                pull.decrypt_ref(&buf[..read]).map_err(|e| {
                    self.state = ChannelState::Errored;
                    e.into()
                })
            }
            _ => Err(NeedsAuthError::new().into()),
        }
    }

    /// Send a `Serialize` message using the underlying `Connection`.
    pub async fn send<T>(&mut self, message: &T) -> Result<usize, ChannelError>
    where
        T: Serialize + Send,
    {
        match &mut self.state {
            ChannelState::Authenticated(_, ref mut push) => {
                let data = push.encrypt(message)?;

                self.connection.write(&data).await.map_err(|e| e.into())
            }
            _ => Err(NeedsAuthError::new().into()),
        }
    }

    /// Authenticate the remote end of this `Channel`
    pub async fn authenticate(&mut self) -> Result<(), ChannelError> {
        let remote_pkey = match &mut self.state {
            ChannelState::ServerPreAuth => {
                self.receive_plain::<PublicKey>().await?
            }
            ChannelState::ClientPreAuth(public) => public.clone(),
            ChannelState::Authenticated(_, _) => return Ok(()),
            ChannelState::Errored => return Err(CorruptedChannel::new().into()),
        };

        let session = self.exchanger.exchange(&remote_pkey)?;
        let (push, pull): (Push, Pull) = session.into();

        self.state = ChannelState::Authenticated(pull, push);

        Ok(())
    }

    /// Checks whether this `Channel` is authenticated
    pub fn is_authenticated(&self) -> bool {
        match &self.state {
            ChannelState::Authenticated(_, _) => true,
            _ => false,
        }
    }

    /// Checks whether this `Channel` is in a usable state
    pub fn is_errored(&self) -> bool {
        match &self.state {
            ChannelState::Errored => true,
            _ => false,
        }
    }
}
