use std::io::{ErrorKind, Result};
use std::net::SocketAddr;

use tokio::net::{lookup_host, ToSocketAddrs};

pub async fn resolve_addr<A: ToSocketAddrs>(addr: A) -> Result<SocketAddr> {
    lookup_host(addr)
        .await?
        .last()
        .ok_or_else(|| ErrorKind::AddrNotAvailable.into())
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn resolve_crates() {
        resolve_addr("crates.io:443").await.expect("resolve failed");

        resolve_addr("invalid.invalid:1293")
            .await
            .expect_err("resolved invalid");
    }
}
