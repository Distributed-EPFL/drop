use std::io::{ErrorKind, Result};
use std::net::SocketAddr;

use super::Socket;

pub use utp::BufferedUtpStream;

impl Socket for BufferedUtpStream {
    fn peer_addr(&self) -> Result<SocketAddr> {
        self.peer_addr()
            .map_or_else(|_| Err(ErrorKind::AddrNotAvailable.into()), Ok)
    }

    fn local_addr(&self) -> Result<SocketAddr> {
        self.local_addr()
            .map_or_else(|_| Err(ErrorKind::AddrNotAvailable.into()), Ok)
    }
}
