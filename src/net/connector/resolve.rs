use std::fmt;
use std::io::ErrorKind;
use std::marker::PhantomData;
use std::net::SocketAddr;

use super::*;

use futures::stream::{FuturesUnordered, StreamExt};

use tokio::net::{self, ToSocketAddrs};

/// A [`Connector`] that uses anything that resolves to a [`SocketAddr`]
/// as a `Candidate`
///
/// [`Connector`]: super::Connector
/// [`SocketAddr`]: std::net::SocketAddr
pub struct ResolveConnector<C, CA> {
    connector: C,
    _c: PhantomData<CA>,
}

impl<C, CA> ResolveConnector<C, CA> {
    /// Create a new `Resolve` [`Connector`] using the given underlying [`Connector`]
    ///
    /// [`Connector`]: super::Connector
    pub fn new(connector: C) -> Self {
        Self {
            connector,
            _c: PhantomData,
        }
    }
}

#[async_trait]
impl<C, CA> Connector for ResolveConnector<C, CA>
where
    C: Connector<Candidate = SocketAddr>,
    CA: ToSocketAddrs + Send + Sync + fmt::Display,
{
    type Candidate = CA;

    async fn establish(
        &self,
        pkey: &PublicKey,
        candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        let candidates = net::lookup_host(candidate).await.context(Io)?;

        let mut futures: FuturesUnordered<_> =
            candidates
                .map(|addr| async move {
                    self.connector.establish(pkey, &addr).await
                })
                .collect();

        while let Some(result) = futures.next().await {
            if result.is_ok() {
                return result;
            }
        }

        Err(ErrorKind::AddrNotAvailable.into()).context(Io)
    }

    fn exchanger(&self) -> &Exchanger {
        self.connector.exchanger()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use crate::net::{Listener, TcpConnector, TcpListener};

    use tokio::task;

    #[tokio::test]
    async fn resolve_connector() {
        let addr = "localhost:3000";
        let exchanger = Exchanger::random();
        let public = *exchanger.keypair().public();

        let mut listener = TcpListener::new(addr, exchanger)
            .await
            .expect("listen failed");

        let connector =
            ResolveConnector::new(TcpConnector::new(Exchanger::random()));

        let handle = task::spawn(async move {
            listener.accept().await.expect("accept failed");
        });

        connector
            .connect(&public, &addr)
            .await
            .expect("connect failed");

        handle.await.expect("listener failure");
    }
}
