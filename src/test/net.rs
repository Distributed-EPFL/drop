pub const LISTENER_ADDR: &str = "localhost";

/// Create two ends of a `Connection` using the specified `Listener`
/// and `Connector` types
#[macro_export]
macro_rules! generate_connection {
    ($listener:ty , $connector:ty) => {
        use std::net::ToSocketAddrs;

        use crate::crypto::key::exchange::{Exchanger, KeyPair};
        use crate::test::net::LISTENER_ADDR;

        use rand;

        let client = KeyPair::random();
        let server = KeyPair::random();
        let client_ex = Exchanger::new(client.clone());
        let server_ex = Exchanger::new(server.clone());

        loop {
            let port: u16 = rand::random();
            let addr: SocketAddr = (LISTENER_ADDR, port)
                .to_socket_addrs()
                .expect("failed to parse localhost")
                .as_slice()[0];
            let mut listener =
                match <$listener>::new(addr, server_ex.clone()).await {
                    Ok(listener) => listener,
                    Err(_) => continue,
                };

            let mut connector = <$connector>::new(client_ex);

            let outgoing = connector
                .connect(server.public(), &addr)
                .await
                .expect("failed to connect");

            let incoming = listener
                .accept()
                .await
                .expect("failed to accept incoming connection");

            assert!(
                incoming.is_secured(),
                "server coulnd't secure the connection"
            );

            assert!(
                outgoing.is_secured(),
                "client couldn't secure the connection"
            );

            return (outgoing, incoming);
        }
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
