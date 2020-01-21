use std::fmt;
use std::net::SocketAddr;

use crate::crypto::key::exchange::PublicKey;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) enum DirectoryRequest {
    Add(DirectoryPeer),
    Fetch(PublicKey),
}

#[derive(Eq, PartialEq, Serialize, Deserialize)]
pub(crate) enum DirectoryResponse {
    /// Add request was a success
    Ok,
    /// Requested peer was found in directory
    Found(SocketAddr),
    /// Requested peer is unknown in the directory
    NotFound(PublicKey),
}

impl fmt::Debug for DirectoryResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ok => "success".to_string(),
                Self::Found(addr) => format!("found at {}", addr),
                Self::NotFound(_) => "not found".to_string(),
            }
        )
    }
}

impl fmt::Display for DirectoryResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        <Self as fmt::Debug>::fmt(self, f)
    }
}

#[derive(Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct DirectoryPeer {
    pkey: PublicKey,
    addr: SocketAddr,
}

impl DirectoryPeer {
    pub fn public(&self) -> &PublicKey {
        &self.pkey
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl From<(PublicKey, SocketAddr)> for DirectoryPeer {
    fn from(info: (PublicKey, SocketAddr)) -> Self {
        let (pkey, addr) = info;
        Self { pkey, addr }
    }
}

#[cfg(test)]
mod test {
    use std::net::{Ipv4Addr, SocketAddr};

    use super::*;

    use crate::crypto::key::exchange::Exchanger;

    #[test]
    fn response_fmt() {
        let pkey = *Exchanger::random().keypair().public();
        let addr: SocketAddr = (Ipv4Addr::UNSPECIFIED, 0).into();
        assert_eq!(format!("{}", DirectoryResponse::Ok), "success");
        assert_eq!(
            format!("{}", DirectoryResponse::NotFound(pkey)),
            "not found"
        );
        assert_eq!(
            format!("{}", DirectoryResponse::Found(addr)),
            format!("found at {}", addr)
        );
    }
}
