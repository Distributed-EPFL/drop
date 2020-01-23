use std::net::SocketAddr;

use super::super::Socket;
use super::{ConnectError, Connector};
use crate::crypto::key::exchange::Exchanger;

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
    use crate::net::listener::tcp::TcpListener;
    use crate::net::listener::Listener;
    use crate::test::next_test_ip4;
    use crate::{exchange_data_and_compare, generate_connection};

    use serde::{Deserialize, Serialize};

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
                "secure channel {} -> {}",
                client.socket.local().unwrap(),
                client.socket.remote().unwrap()
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
}
