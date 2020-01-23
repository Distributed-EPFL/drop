use std::fmt;
use std::net::SocketAddr;

use super::super::Connection;
use super::{Listener, ListenerError};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use tokio::net::{TcpListener as TokioListener, ToSocketAddrs};

use tracing::{debug, debug_span, info};
use tracing_futures::Instrument;

/// A plain `TcpListener` that accepts connections on a given IP address and
/// port
pub struct TcpListener {
    listener: TokioListener,
    exchanger: Exchanger,
}

impl TcpListener {
    /// Create a new `TcpListener` that will listen on the candidate address
    ///
    /// # Arguments
    ///
    /// * `candidate` - The target address to listen on
    /// * `exchanger` - A key `Exchanger` to be used when handshaking with the
    /// remote end
    ///
    /// # Example
    /// ```
    /// use std::net::{Ipv4Addr, SocketAddr};
    /// use drop::crypto::key::exchange::Exchanger;
    /// use drop::net::listener::TcpListener;
    ///
    /// let addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, 0).into();
    /// let listener = TcpListener::new(addr, Exchanger::random());
    /// ```
    pub async fn new<A: ToSocketAddrs + fmt::Display>(
        candidate: A,
        exchanger: Exchanger,
    ) -> Result<Self, ListenerError> {
        debug!(
            "listening with TCP on {} with {}",
            candidate,
            exchanger.keypair().public()
        );
        TokioListener::bind(candidate)
            .await
            .map(|listener| Self {
                listener,
                exchanger,
            })
            .map_err(|e| e.into())
    }
}

#[cfg(unix)]
mod unix {
    use std::os::unix::io::{AsRawFd, RawFd};

    use super::TcpListener;

    impl AsRawFd for TcpListener {
        fn as_raw_fd(&self) -> RawFd {
            self.listener.as_raw_fd()
        }
    }
}

#[async_trait]
impl Listener for TcpListener {
    type Candidate = SocketAddr;

    async fn candidates(&self) -> Result<&[Self::Candidate], ListenerError> {
        todo!()
    }

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr().ok()
    }

    /// Accept an incoming `Connection` from this `TcpListener` and performs
    /// key exchange to authenticate the remote peer.
    async fn accept(&mut self) -> Result<Connection, ListenerError> {
        let (stream, remote) = self.listener.accept().await?;

        info!("incoming tcp connection from {}", remote);

        let mut connection = Connection::new(Box::new(stream));

        connection
            .secure_client(&self.exchanger)
            .instrument(debug_span!("key_exchange"))
            .await?;

        info!("accepted secure connection from {}", remote);

        Ok(connection)
    }
}

impl fmt::Display for TcpListener {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let addr = self.local_addr().map_or(Err(fmt::Error), Ok)?;

        write!(f, "tcp listener on {}", addr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test::next_test_ip4;

    #[tokio::test]
    #[should_panic]
    async fn tcp_double_bind() {
        let exchanger = Exchanger::random();
        let addr = next_test_ip4();
        let one = TcpListener::new(addr, exchanger.clone())
            .await
            .expect("failed to bind");

        let two = TcpListener::new(addr, exchanger)
            .await
            .expect("failed to bind");

        assert_eq!(one.local_addr().unwrap(), two.local_addr().unwrap());
    }
}
