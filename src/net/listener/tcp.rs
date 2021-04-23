use std::fmt;
use std::net::SocketAddr;

use super::super::socket::Socket;
use super::{Io, Listener, ListenerError};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use snafu::ResultExt;

use tokio::net::{TcpListener as TokioListener, ToSocketAddrs};

use tracing::{debug, debug_span, info};
use tracing_futures::Instrument;

/// A plain `TcpListener` that accepts connections on a given IP address and
/// port
pub struct Direct {
    listener: TokioListener,
    exchanger: Exchanger,
}

impl Direct {
    /// Create a new `TcpListener` that will listen on the candidate address
    ///
    /// # Arguments
    ///
    /// * `candidate` The target address to listen on
    /// * `exchanger` A key `Exchanger` to be used when handshaking with the
    /// remote end
    ///
    /// # Example
    /// ```
    /// use std::net::{Ipv4Addr, SocketAddr};
    /// use drop::crypto::key::exchange::Exchanger;
    /// use drop::net::TcpListener;
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
            .context(Io)
    }
}

#[cfg(unix)]
mod unix {
    use std::os::unix::io::{AsRawFd, RawFd};

    use super::Direct;

    impl AsRawFd for Direct {
        fn as_raw_fd(&self) -> RawFd {
            self.listener.as_raw_fd()
        }
    }
}

#[async_trait]
impl Listener for Direct {
    type Candidate = SocketAddr;

    async fn candidates(&self) -> Result<Vec<Self::Candidate>, ListenerError> {
        Ok(vec![self.listener.local_addr().context(Io)?])
    }

    fn local_addr(&self) -> Option<SocketAddr> {
        self.listener.local_addr().ok()
    }

    /// Accept an incoming `Connection` from this `TcpListener` and performs
    /// key exchange to authenticate the remote peer.
    async fn establish(&mut self) -> Result<Box<dyn Socket>, ListenerError> {
        let (stream, remote) = self
            .listener
            .accept()
            .instrument(debug_span!("tcp_accept"))
            .await
            .context(Io)?;

        info!("incoming tcp connection from {}", remote);

        Ok(Box::new(stream))
    }

    fn exchanger(&self) -> &Exchanger {
        &self.exchanger
    }
}

impl fmt::Display for Direct {
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
        let one = Direct::new(addr, exchanger.clone())
            .await
            .expect("failed to bind");

        let two = Direct::new(addr, exchanger).await.expect("failed to bind");

        assert_eq!(one.local_addr().unwrap(), two.local_addr().unwrap());
    }

    #[tokio::test]
    async fn tcp_listener_addr() {
        let addr = next_test_ip4();
        let listener = Direct::new(addr, Exchanger::random())
            .await
            .expect("bind failed");

        assert_eq!(
            listener.local_addr().unwrap(),
            addr,
            "wrong listen address"
        );
    }
}
