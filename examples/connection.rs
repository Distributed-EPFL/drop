use std::net::{Ipv4Addr, SocketAddr};

use drop::crypto::key::exchange::Exchanger;
use drop::net::{Connector, Listener, TcpConnector, TcpListener};

use tokio::task;

const MESSAGE_COUNT: usize = 2000;

#[tokio::main]
async fn main() {
    // first creating two sets of keys to exchange messages in a secure fashion
    let (client, server) = (Exchanger::random(), Exchanger::random());
    let public = *server.keypair().public();

    let addr: SocketAddr = (Ipv4Addr::LOCALHOST, 2048).into();

    // now creating both the listener that will accept connections and a connector to
    // connect to that listener
    let connector = TcpConnector::new(client);
    let listener = TcpListener::new(addr, server).await.expect("listen failed");

    // establishing the actual connection
    let mut connection = connector
        .connect(&public, &addr)
        .await
        .expect("failed to connect");

    let messages = 0..MESSAGE_COUNT;

    // we spawn the receiver task that will accept the connection and receive messages
    let handle = task::spawn(do_receive(listener));

    // creating a set of futures that will send the messages
    for i in messages {
        connection.send(&i).await.expect("send failed");
    }

    // closing the connection so that the receiver won't hang forever
    connection.close().await.expect("close failed");

    // wait for the receiver to process all messages
    handle.await.expect("listener failed");
}

async fn do_receive<L: Listener>(mut listener: L) {
    let mut connection = listener.accept().await.expect("accept failed");

    while let Ok(message) = connection.receive::<usize>().await {
        println!(
            "received message {} from {}",
            message,
            connection.remote_key().unwrap()
        );
    }
}
