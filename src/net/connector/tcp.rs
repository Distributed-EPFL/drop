use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::super::{Connection, TokioError};
use crate as drop;
use crate::crypto::stream::Push as KeyExchanger;
use crate::crypto::Key as PublicKey;

use macros::error;

use tokio::net::{TcpStream, ToSocketAddrs};

/// A `Connector` that uses direct TCP connections.
pub struct TcpDirect {}

impl TcpDirect {
    /// Asynchronously connect to a given peer using its `PublicKey` to
    /// authenticate them.
    pub fn connect<A: ToSocketAddrs>(
        addr: A,
        public: &PublicKey,
    ) -> Box<dyn Future<Output = dyn Connection<Error = TcpError>>> {
        unimplemented!()
    }
}

impl Future for TcpDirect {
    type Output = TcpConn;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        unimplemented!()
    }
}

pub struct TcpConn {
    stream: TcpStream,
    exchanger: KeyExchanger,
}

impl TcpConn {}

error! {
    type: TcpError,
    description: "tcp connection error",
    causes: (TokioError),
}
