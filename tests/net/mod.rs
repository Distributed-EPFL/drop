use std::env;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicU16, Ordering};

use drop::crypto::key::exchange::Exchanger;
use drop::net::{
    Connector, DirectoryConnector, DirectoryListener, DirectoryServer,
    Listener, TcpConnector, TcpListener,
};

use tokio::task;

use tracing::trace_span;
use tracing_futures::Instrument;
use tracing_subscriber::FmtSubscriber;

/// Initialize an asynchronous logger for test environment
pub fn init_logger() {
    if let Some(level) = env::var("RUST_LOG").ok().map(|x| x.parse().ok()) {
        let subscriber =
            FmtSubscriber::builder().with_max_level(level).finish();

        let _ = tracing::subscriber::set_global_default(subscriber);
    }
}

fn next_test_ip4() -> SocketAddr {
    static PORT_OFFSET: AtomicU16 = AtomicU16::new(0);

    (
        "127.0.0.1".parse::<Ipv4Addr>().unwrap(),
        8000 + PORT_OFFSET.fetch_add(1, Ordering::AcqRel),
    )
        .into()
}

#[tokio::test]
async fn directory_server_blackbox() {
    init_logger();
    let dir_addr = next_test_ip4();
    let addr = next_test_ip4();
    let dir_exchanger = Exchanger::random();
    let dir_pkey = *dir_exchanger.keypair().public();

    let listener = TcpListener::new(dir_addr, dir_exchanger)
        .instrument(trace_span!("directory_bind"))
        .await
        .expect("directory bind failed");

    let (server, exit) = DirectoryServer::new(listener);
    let server_handle =
        task::spawn(async move { server.serve().await.expect("serve failed") });

    let node_exchanger = Exchanger::random();
    let node_public = *node_exchanger.keypair().public();
    let connector = TcpConnector::new(Exchanger::random());
    let listener = TcpListener::new(addr, node_exchanger)
        .instrument(trace_span!("node_listen"))
        .await
        .expect("node_bind failed");
    let mut directory_listener =
        DirectoryListener::new(listener, connector, dir_addr)
            .instrument(trace_span!("directory_listen"))
            .await
            .expect("node_register failed");

    let listener_handle = task::spawn(async move {
        let mut connection =
            directory_listener.accept().await.expect("accept failed");

        let data: u32 = connection.receive().await.expect("recv failed");

        assert_eq!(data, 0, "corrupted data");
    });

    let connector =
        DirectoryConnector::new(TcpConnector::new(Exchanger::random()));

    let client_handle = task::spawn(async move {
        let dir_info = (dir_pkey, dir_addr).into();
        let mut connection = connector
            .connect(&node_public, &dir_info)
            .await
            .expect("connect failed");

        connection.send(&0u32).await.expect("send failed");
    });

    client_handle.await.expect("client failure");
    listener_handle.await.expect("listener failure");

    exit.send(()).expect("server died prematurely");

    server_handle.await.expect("server failure");
}
