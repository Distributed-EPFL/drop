use std::io::Result;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::Socket;

use tokio::io::{AsyncRead, AsyncWrite, BufReader};

use utp::UtpStream;

const MTU: usize = 1500;

/// A buffered `UtpStream` to avoid making too many system calls when sending
/// small amounts of data through utp
pub struct BufferedUtpStream {
    stream: BufReader<UtpStream>,
}

impl BufferedUtpStream {
    pub(crate) fn new(stream: UtpStream) -> Self {
        Self {
            stream: BufReader::with_capacity(MTU, stream),
        }
    }

    fn get_stream(self: Pin<&mut Self>) -> Pin<&mut BufReader<UtpStream>> {
        unsafe { self.map_unchecked_mut(|s| &mut s.stream) }
    }
}

impl Socket for BufferedUtpStream {
    fn remote(&self) -> Result<SocketAddr> {
        Ok(self.stream.get_ref().peer_addr())
    }

    fn local(&self) -> Result<SocketAddr> {
        Ok(self.stream.get_ref().local_addr())
    }
}

impl AsyncRead for BufferedUtpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &mut [u8],
    ) -> Poll<Result<usize>> {
        // bypass buffering when reading since UtpStream already buffers
        self.get_stream().get_pin_mut().poll_read(cx, buf)
    }
}

impl AsyncWrite for BufferedUtpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize>> {
        self.get_stream().poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Result<()>> {
        self.get_stream().poll_flush(cx)
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut Context,
    ) -> Poll<Result<()>> {
        self.get_stream().poll_shutdown(cx)
    }
}
