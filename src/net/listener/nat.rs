use std::io::{Error, ErrorKind};
use std::net::{Ipv4Addr, SocketAddrV4};

use super::super::socket::Socket;
use super::{Listener, ListenerError, TcpListener};
use crate::crypto::key::exchange::Exchanger;

use async_trait::async_trait;

use igd::aio::{search_gateway, Gateway};
use igd::{
    AddPortError, GetExternalIpError, PortMappingProtocol, SearchError,
    SearchOptions,
};

/// Default lease duration for port mapping
pub const PMP_LEASE_DURATION: u32 = 0;

/// A `Listener` that uses UPnP to open a publicly reachable port to accept
/// incoming `Connection`s
pub struct IgdListener {
    listener: TcpListener,
    public: SocketAddrV4,
}

impl IgdListener {
    /// Create a new `IgdListener` that will try to map a port before listening
    /// before accepting `Connection`s.
    pub async fn new(
        exchanger: Exchanger,
        addr: SocketAddrV4,
    ) -> Result<Self, ListenerError> {
        let mut gateway = search_gateway(SearchOptions::default()).await?;

        if let Ok(_) = Self::open_external_port(&mut gateway, addr).await {
            let public = SocketAddrV4::new(
                Self::public_ip(&mut gateway).await?,
                addr.port(),
            );

            let listener = TcpListener::new(addr, exchanger).await?;

            Ok(Self { listener, public })
        } else {
            let err: Error = ErrorKind::AddrNotAvailable.into();
            Err(err.into())
        }
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
            .await?;
        Ok(())
    }
}

impl From<AddPortError> for ListenerError {
    fn from(err: AddPortError) -> Self {
        match err {
            AddPortError::PortInUse => todo!(),
            _ => todo!(),
        }
    }
}

impl From<GetExternalIpError> for ListenerError {
    fn from(err: GetExternalIpError) -> Self {
        match err {
            _ => todo!(),
        }
    }
}

impl From<SearchError> for ListenerError {
    fn from(err: SearchError) -> Self {
        match err {
            _ => todo!(),
        }
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
