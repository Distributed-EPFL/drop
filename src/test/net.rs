use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU16, Ordering};

use crate::crypto::key::exchange::Exchanger;

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

/// Generate a set of `count` address and port pairs for local testing
pub fn test_addrs(count: usize) -> Vec<(Exchanger, SocketAddr)> {
    (0..count)
        .map(|_| (Exchanger::random(), next_test_ip4()))
        .collect()
}

/// Create two ends of a `Connection` using the specified `Listener`
/// and `Connector` types
#[macro_export]
macro_rules! generate_connection {
    ($listener:ty , $connector:ty) => {
        use crate::crypto::key::exchange::{Exchanger, KeyPair};
        use crate::net::Connector;

        let client = KeyPair::random();
        let server = KeyPair::random();
        let client_ex = Exchanger::new(client.clone());
        let server_ex = Exchanger::new(server.clone());

        let addr = next_test_ip4();
        let mut listener = <$listener>::new(addr, server_ex.clone())
            .await
            .expect("listen failed");

        let connector = <$connector>::new(client_ex);

        let handle = tokio::task::spawn(async move {
            listener
                .accept()
                .await
                .expect("failed to accept incoming connection")
        });

        let outgoing = connector
            .connect(server.public(), &addr)
            .await
            .expect("failed to connect");

        let incoming = handle.await.expect("task failure");

        assert!(
            incoming.is_secured(),
            "server coulnd't secure the connection"
        );

        assert!(
            outgoing.is_secured(),
            "client couldn't secure the connection"
        );

        return (outgoing, incoming);
    };
}

/// Exchanges the given data using a new `Connection` and checks that the
/// received data is the same as what was sent.
#[macro_export]
macro_rules! exchange_data_and_compare {
    ($data:expr, $type:ty, $setup:ident) => {
        let (mut client, mut listener) = $setup().await;

        let data = $data;

        client.send(&data).await.expect("failed to send");

        let recvd: $type = listener.receive().await.expect("failed to receive");

        assert_eq!(data, recvd, "data is not the same");
    };
}
