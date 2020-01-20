use std::io::Result;
use std::net::SocketAddr;

use super::Socket;

use tokio::net::TcpStream;

impl Socket for TcpStream {
    fn local(&self) -> Result<SocketAddr> {
        self.local_addr()
    }

    fn remote(&self) -> Result<SocketAddr> {
        self.peer_addr()
    }
}
