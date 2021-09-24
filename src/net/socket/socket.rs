use std::io;
use std::net::SocketAddr;

use tokio::io::{AsyncRead, AsyncWrite};

/// Trait for structs that are able to asynchronously send and receive data from
/// the network. Only requirement is asynchronously reading and writing arrays
/// of bytes
pub trait Socket: AsyncRead + AsyncWrite + Unpin + Send + Sync {
    /// Address of the remote peer for this `Connection`
    fn peer_addr(&self) -> io::Result<SocketAddr>;

    /// Local address in use by this `Connection`
    fn local_addr(&self) -> io::Result<SocketAddr>;
}
