use std::fmt;
use std::net::SocketAddr;

use crate::crypto::key::exchange::PublicKey;
use crate::message;

#[message]
#[derive(Copy, Eq, PartialEq)]
pub enum Request {
    /// Add this peer to the directory
    Add(Info),
    /// Fetch a peer from the directory by its public key
    Fetch(PublicKey),
    /// Wait for a number of peer to be registered on the directory
    Wait(usize),
}

#[message]
#[derive(Eq, PartialEq)]
/// A response used by the directory protocol
pub enum Response {
    /// Add request was a success
    Ok,
    /// Requested peer was found in directory
    Found(PublicKey, SocketAddr),
    /// Requested peer is unknown in the directory
    NotFound(PublicKey),
}

impl fmt::Display for Response {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Ok => "success".to_string(),
                Self::Found(pkey, addr) =>
                    format!("found {} at {}", pkey, addr),
                Self::NotFound(_) => "not found".to_string(),
            }
        )
    }
}

#[message]
#[derive(Copy, Hash, Eq, PartialEq)]
/// Information needed to connect to a remote peer.
pub struct Info {
    pkey: PublicKey,
    addr: SocketAddr,
}

impl Info {
    /// Get the `PublicKey` contained in this `Info`
    pub fn public(&self) -> &PublicKey {
        &self.pkey
    }

    /// Get the remote `SocketAddr` that can be used to connect
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

impl fmt::Display for Info {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} at {}", self.pkey, self.addr)
    }
}

impl From<(PublicKey, SocketAddr)> for Info {
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
        assert_eq!(format!("{}", Response::Ok), "success");
        assert_eq!(format!("{}", Response::NotFound(pkey)), "not found");
        assert_eq!(
            format!("{}", Response::Found(pkey, addr)),
            format!("found {} at {}", pkey, addr)
        );
    }
}
