use std::io::Error as IoError;
use std::mem;
use std::net::SocketAddr;

use super::super::{Connection, TokioError};
use super::Connector;
use crate as drop;
use crate::crypto::stream::{Pull, Push as KeyExchanger, Push};
use crate::crypto::EncryptError;
use crate::crypto::Key as PublicKey; // placeholder for real public key
use crate::error::Error;

use async_trait::async_trait;

use bincode::{deserialize, serialize, ErrorKind as BincodeErrorKind};

use macros::error;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

pub type BincodeError = Box<BincodeErrorKind>;

error! {
    type: TcpError,
    description: "tcp connection error",
    causes: (BincodeError, TokioError, IoError, EncryptError),
}

/// A `Connector` that uses direct TCP connections to a remote peer
pub struct TcpDirect {}

#[async_trait]
impl Connector for TcpDirect {
    type Addr = SocketAddr;

    type Connection = TcpConn;

    type Error = TcpError;

    async fn connect(
        addrs: Self::Addr,
        exchanger: KeyExchanger,
        pkey: &PublicKey,
    ) -> Result<Self::Connection, Self::Error> {
        let stream = TcpStream::connect(addrs).await?;

        Ok(TcpConn::new(stream, exchanger, pkey.clone()))
    }
}

enum TcpState {
    Connected,
    Authenticated(Pull, Push),
    Errored(TcpError),
}

/// An asynchronous Tcp connection
pub struct TcpConn {
    stream: TcpStream,
    exchanger: KeyExchanger,
    public: PublicKey,
    state: TcpState,
}

impl TcpConn {
    fn new(
        stream: TcpStream,
        exchanger: KeyExchanger,
        public: PublicKey,
    ) -> Self {
        Self {
            stream,
            exchanger,
            public,
            state: TcpState::Connected,
        }
    }

    fn authenticate(&mut self) -> Result<(), TcpError> {
        unimplemented!()
    }
}

#[async_trait]
impl Connection for TcpConn {
    type Error = TcpError;

    async fn receive(&mut self, buf: &mut [u8]) -> Result<usize, TcpError> {
        match &self.state {
            TcpState::Connected => {
                self.authenticate()?;
                self.receive(buf).await
            }
            TcpState::Authenticated(pull, push) => {
                let read = match self.stream.read(buf).await {
                    Ok(read) => read,
                    Err(e) => return Err(TcpError::from(e)),
                };

                match deserialize(&buf[..read]) {
                    Ok(value) => Ok(value),
                    Err(e) => Err(TcpError::from(e)),
                }
            }
            TcpState::Errored(e) => Err(*e),
        }
    }

    async fn send(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        match &self.state {
            TcpState::Connected => {
                self.authenticate()?;
                self.send(buf).await
            }
            TcpState::Authenticated(_, mut push) => {
                let encrypted = push.encrypt(&buf)?;

                match self.stream.write(&encrypted).await {
                    Ok(written) => Ok(written),
                    Err(e) => Err(TcpError::from(e)),
                }
            }
            TcpState::Errored(e) => Err(*e),
        }
    }
}
