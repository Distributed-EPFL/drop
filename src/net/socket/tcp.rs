use crate::net::socket::Socket;

use std::io;
use std::net::SocketAddr;

use tokio::net::TcpStream;

impl Socket for TcpStream {
    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.local_addr()
    }

    fn peer_addr(&self) -> io::Result<SocketAddr> {
        self.peer_addr()
    }
}
