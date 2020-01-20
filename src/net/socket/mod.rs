/// Tcp `Socket` implementation
pub mod tcp;
/// uTp `Socket` implementation
pub mod utp;

use std::io::Result;
use std::net::SocketAddr;

use tokio::io::{AsyncRead, AsyncWrite};

/// Trait for structs that are able to asynchronously send and receive data from
/// the network. Only requirement is asynchronously reading and writing arrays
/// of bytes
pub trait Socket: AsyncRead + AsyncWrite + Unpin + Send + Sync {
    /// Address of the remote peer for this `Connection`
    fn remote(&self) -> Result<SocketAddr>;

    /// Local address in use by this `Connection`
    fn local(&self) -> Result<SocketAddr>;
}
