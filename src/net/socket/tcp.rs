use std::io::Result;
use std::net::SocketAddr;

use super::Socket;

use tokio::net::TcpStream;

impl Socket for TcpStream {
    fn local_addr(&self) -> Result<SocketAddr> {
        self.local_addr()
    }

    fn peer_addr(&self) -> Result<SocketAddr> {
        self.peer_addr()
    }
}
