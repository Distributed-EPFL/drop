use std::future::Future;

use crate as drop;
use macros::error;

use serde::{Deserialize, Serialize};

/// Connectors are used to establish connection to remote peers
/// in a secure manner.
pub mod connector;

/// An abstract `Connection` used to send and receive data from peers
pub trait Connection {
    /// The type of `Error` that this `Connection` returns when unable to
    /// send data
    type Error;

    /// Send a `Serialize` message asynchronously onto the network using this
    /// `Connection`.
    fn send<T: Serialize>(
        &mut self,
        msg: &T,
    ) -> Box<dyn Future<Output = Result<(), Self::Error>>> {
        self.send_async(msg)
    }

    /// Send a `Serialize` message onto the network. This method is not allowed
    /// to block in any case
    fn send_async<T: Serialize>(
        &mut self,
        msg: &T,
    ) -> Box<dyn Future<Output = Result<(), Self::Error>>>;

    /// Send a `Serialize` message through this `Connection`. <br />
    /// This method is allowed to block if the message can't be sent immediately.
    fn send_sync<T: Serialize>(
        &mut self,
        msg: &T,
    ) -> Box<dyn Future<Output = Result<(), Self::Error>>>;

    /// Receive a `Deserialize message from the network asynchronously
    fn receive<T>(&mut self) -> Box<dyn Future<Output = Result<T, Self::Error>>>
    where
        T: for<'de> Deserialize<'de> + Sized,
    {
        self.receive_async()
    }

    /// Receive a `Deserialize message from the network asynchronously
    fn receive_async<T>(
        &mut self,
    ) -> Box<dyn Future<Output = Result<T, Self::Error>>>;

    /// Receive a `Deserialize` message from this `Connection`. This method will
    /// block if no data is available on this `Connection`
    fn receive_sync<T>(
        &mut self,
    ) -> Box<dyn Future<Output = Result<T, Self::Error>>>
    where
        for<'de> T: Deserialize<'de> + Sized;
}

error! {
    type: TokioError,
    description: "tokio encountered an error",
}
