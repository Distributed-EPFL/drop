mod log;
pub use log::*;

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU16, Ordering};

/// Get the next available port for testing purposes
pub fn next_test_port() -> u16 {
    static PORT_OFFSET: AtomicU16 = AtomicU16::new(0);
    const PORT_START: u16 = 9600;

    PORT_START + PORT_OFFSET.fetch_add(1, Ordering::Relaxed)
}

/// Get the next available `SocketAddr` that can be used for testing
pub fn next_test_ip4() -> SocketAddr {
    (Ipv4Addr::new(127, 0, 0, 1), next_test_port()).into()
}
