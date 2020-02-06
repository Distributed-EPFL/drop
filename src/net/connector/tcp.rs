use std::net::SocketAddr;

use super::super::Socket;
use super::{ConnectError, Connector};
use crate::crypto::key::exchange::{Exchanger, PublicKey};

use async_trait::async_trait;

use tokio::net::TcpStream;

use tracing::info;

/// A `Connector` that uses direct TCP connections to a remote peer
pub struct TcpDirect {
    exchanger: Exchanger,
}

impl TcpDirect {
    /// Create a new `TcpDirect` `Connector` using the given
    /// `Exchanger` to compute shared secrets
    ///
    /// # Arguments
    /// * `exchanger` - The key exchanger to be used when handshaking with
    /// remote peers
    pub fn new(exchanger: Exchanger) -> Self {
        Self { exchanger }
    }
}

#[async_trait]
impl Connector for TcpDirect {
    /// This `Connector` uses a pair of `IpAddr` and port as destination
    type Candidate = SocketAddr;

    /// Returns the local client's `Exchanger`
    fn exchanger(&self) -> &Exchanger {
        &self.exchanger
    }

    /// Open a `Socket` to the specified destination using TCP
    async fn establish(
        &mut self,
        _: &PublicKey,
        candidate: &Self::Candidate,
    ) -> Result<Box<dyn Socket>, ConnectError> {
        info!("establishing tcp connection to {}", candidate);

        let stream: Box<dyn Socket> =
            Box::new(TcpStream::connect(candidate).await?);

        Ok(stream)
    }
}

#[cfg(test)]
mod test {
    use super::super::Connection;
    use super::*;
    use crate::crypto::key::exchange::PublicKey;
    use crate::net::listener::Listener;
    use crate::net::listener::TcpListener;
    use crate::test::next_test_ip4;
    use crate::{exchange_data_and_compare, generate_connection};

    use serde::{Deserialize, Serialize};

    use tokio::task;

    pub async fn setup_tcp() -> (Connection, Connection) {
        generate_connection!(TcpListener, TcpDirect);
    }

    #[tokio::test]
    async fn tcp_u8_exchange() {
        exchange_data_and_compare!(0, u8, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_u16_exchange() {
        exchange_data_and_compare!(0, u16, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_u32_exchange() {
        exchange_data_and_compare!(0, u32, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_u64_exchange() {
        exchange_data_and_compare!(0, u64, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_i8_exchange() {
        exchange_data_and_compare!(0, i8, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_i16_exchange() {
        exchange_data_and_compare!(0, i16, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_i32_exchange() {
        exchange_data_and_compare!(0, i32, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_i64_exchange() {
        exchange_data_and_compare!(0, i64, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_struct_exchange() {
        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct T {
            a: u32,
            b: u64,
            c: A,
        }

        #[derive(Debug, PartialEq, Serialize, Deserialize)]
        struct A {
            a: u8,
            b: u16,
        }

        let data = T {
            a: 258,
            b: 30567,
            c: A { a: 66, b: 245 },
        };

        exchange_data_and_compare!(data, T, setup_tcp);
    }

    #[tokio::test]
    async fn tcp_hashmap_exchange() {
        use std::collections::HashMap;

        let mut hashmap: HashMap<u32, u128> = HashMap::default();

        for _ in 0..rand::random::<usize>() % 2048 {
            hashmap.insert(rand::random(), rand::random());
        }

        exchange_data_and_compare!(hashmap, HashMap<u32, u128>, setup_tcp);
    }

    #[tokio::test]
    async fn garbage_data_decryption() {
        let (mut client, mut listener) = setup_tcp().await;

        client
            .send_plain(&0u32)
            .await
            .expect("failed to send unencrypted data");

        listener
            .receive::<u32>()
            .await
            .expect_err("received garbage correctly");

        assert!(
            listener.is_broken(),
            "incorrect state for listener connection"
        );
    }

    #[tokio::test]
    async fn initial_state() {
        let (client, listener) = setup_tcp().await;

        assert!(client.is_secured(), "client is not authenticated");
        assert!(listener.is_secured(), "listener is not authenticated");
        assert!(!listener.is_broken(), "listener is errored");
        assert!(!client.is_broken(), "client is errored");
    }

    #[tokio::test]
    async fn connection_fmt() {
        let (client, _listener) = setup_tcp().await;

        assert_eq!(
            format!("{:?}", client),
            format!(
                "secure connection {} -> {}",
                client.socket.local_addr().unwrap(),
                client.socket.peer_addr().unwrap()
            )
        );
    }

    #[tokio::test]
    async fn tcp_non_existent() {
        let exchanger = Exchanger::random();
        let mut connector = TcpDirect::new(exchanger.clone());
        let addr = next_test_ip4();

        connector
            .connect(exchanger.keypair().public(), &addr)
            .await
            .expect_err("connected to non-existent listener");
    }

    #[tokio::test]
    async fn corrupted_connection() {
        let srv = next_test_ip4();
        let mut listener = TcpListener::new(srv, Exchanger::random())
            .await
            .expect("bind failed");
        let mut connector = TcpDirect::new(Exchanger::random());

        let handle = task::spawn(async move {
            let mut bad_conn = listener.accept().await.expect("accept failed");
            bad_conn
                .receive::<u32>()
                .await
                .expect_err("wrong decryption");

            assert!(bad_conn.is_broken(), "connection is not broken");
            assert!(!bad_conn.is_secured(), "connection is still secured");

            bad_conn
                .send(&0u32)
                .await
                .expect_err("send succeded on broken connection");
            bad_conn
                .receive::<u32>()
                .await
                .expect_err("recv success on broken connection");
        });

        let wrong_keypair = Exchanger::random();
        let mut bad_conn = connector
            .connect(wrong_keypair.keypair().public(), &srv)
            .await
            .expect("connect failed");

        bad_conn.send(&0u32).await.expect("send failed");

        handle.await.expect("listener failure");
    }

    #[tokio::test]
    async fn unsecured_connection() {
        use tokio::io::AsyncWriteExt;

        let srv = next_test_ip4();

        let exchanger = Exchanger::random();
        let srv_pub = *exchanger.keypair().public();
        let mut listener = TcpListener::new(srv, exchanger)
            .await
            .expect("listen failed");
        let mut connector = TcpDirect::new(Exchanger::random());

        let handle = task::spawn(async move {
            listener.accept().await.expect_err("accept suceeded");
        });

        let mut socket = connector
            .establish(&srv_pub, &srv)
            .await
            .expect("connect failed");

        socket
            .write(&[0u8; std::mem::size_of::<PublicKey>()])
            .await
            .expect("write failed");

        let mut connection = Connection::new(socket);

        assert!(!connection.is_secured(), "connection is secured");

        connection
            .send(&0u32)
            .await
            .expect_err("send on insecure connection");

        handle.await.expect("listener failure");
    }
}
