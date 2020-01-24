use std::io::{Error, ErrorKind};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::Duration;

use super::super::socket::Socket;
use super::{Listener, ListenerError, TcpListener};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use igd::aio::{search_gateway, Gateway};
use igd::{
    AddPortError, GetExternalIpError, PortMappingProtocol, SearchError,
    SearchOptions,
};

use tokio::sync::oneshot::error::TryRecvError;
use tokio::sync::oneshot::{channel, Sender};
use tokio::task;
use tokio::time::delay_for;

use tracing::{debug_span, info, info_span};
use tracing_futures::Instrument;

/// Default lease duration for port mapping
pub const PMP_LEASE_DURATION: u32 = 3600;

/// A `Listener` that uses UPnP to open a publicly reachable port to accept
/// incoming `Connection`s
pub struct IgdListener {
    listener: TcpListener,
    public: SocketAddrV4,
    sender: Sender<()>,
}

impl IgdListener {
    /// Create a new `IgdListener` that will try to map a publicly reachable
    /// port before listening before accepting `Connection`s.
    /// This also starts an async task that will periodically renew the port
    /// mapping lease with the gateway.
    pub async fn new(
        exchanger: Exchanger,
        addr: SocketAddrV4,
    ) -> Result<Self, ListenerError> {
        let mut gateway = search_gateway(SearchOptions::default())
            .instrument(debug_span!("gateway_search"))
            .await?;

        info!("found upnp device {}", gateway);

        Self::open_external_port(&mut gateway, addr).await?;

        let public = SocketAddrV4::new(
            Self::public_ip(&mut gateway).await?,
            addr.port(),
        );

        info!("binding public {} with TCP", public);

        let listener = TcpListener::new(addr, exchanger).await?;

        let sender =
            Self::schedule_lease_rebind(gateway, PMP_LEASE_DURATION, public);

        Ok(Self {
            listener,
            public,
            sender,
        })
    }

    fn schedule_lease_rebind(
        gateway: Gateway,
        timeout: u32,
        addr: SocketAddrV4,
    ) -> Sender<()> {
        let (tx, mut rx) = channel();

        task::spawn(async move {
            loop {
                delay_for(Duration::from_secs(timeout.into())).await;

                match rx.try_recv() {
                    Ok(_) | Err(TryRecvError::Closed) => break,
                    _ => continue,
                }
            }
        })
        .instrument(info_span!("pmp_renew"));

        tx
    }

    /// Close this `IgdListener` and stops the port mapping lease renewer
    pub async fn close(self) {
        // the renewer may have already died but we don't care
        let _ = self.sender.send(());
    }

    async fn public_ip(
        gateway: &mut Gateway,
    ) -> Result<Ipv4Addr, GetExternalIpError> {
        gateway.get_external_ip().await
    }

    async fn open_external_port(
        gateway: &mut Gateway,
        addr: SocketAddrV4,
    ) -> Result<(), ListenerError> {
        gateway
            .add_port(
                PortMappingProtocol::TCP,
                addr.port(),
                addr,
                PMP_LEASE_DURATION,
                "drop",
            )
            .instrument(debug_span!("port_map"))
            .await?;

        info!("mapped external address {}", addr);

        Ok(())
    }
}

impl From<AddPortError> for ListenerError {
    fn from(err: AddPortError) -> Self {
        Error::new(ErrorKind::Other, format!("error adding mapping: {}", err))
            .into()
    }
}

impl From<GetExternalIpError> for ListenerError {
    fn from(err: GetExternalIpError) -> Self {
        Error::new(
            ErrorKind::Other,
            format!("error getting public ip: {}", err),
        )
        .into()
    }
}

impl From<SearchError> for ListenerError {
    fn from(err: SearchError) -> Self {
        Error::new(ErrorKind::Other, format!("gateway search error: {}", err))
            .into()
    }
}

#[async_trait]
impl Listener for IgdListener {
    type Candidate = SocketAddrV4;

    async fn accept_raw(&mut self) -> Result<Box<dyn Socket>, ListenerError> {
        self.listener.accept_raw().await
    }

    async fn candidates(&self) -> Result<Vec<Self::Candidate>, ListenerError> {
        Ok(vec![self.public])
    }

    fn exchanger(&self) -> &Exchanger {
        self.listener.exchanger()
    }
}
