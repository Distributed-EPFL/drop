mod directory;
pub use self::directory::*;

use std::io::Error;

use super::{ListenerError, ReceiveError, SendError};

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
/// Errors encountered by servers
pub enum ServerError {
    /// I/O error encountered by the server
    #[snafu(display("i/o error when {}: {}", when, source))]
    ServerIo {
        /// Details about the step that failed
        when: String,
        /// Underlying error cause
        source: Error,
    },
    #[snafu(display("error receiving data when {}: {}", when, source))]
    /// Error receiving data from client
    Receive {
        /// Details about step that failed
        when: String,
        /// Underlying error cause
        source: ReceiveError,
    },
    #[snafu(display("error sending data while {}: {}", when, source))]
    /// Error sending data to requesting client
    Send {
        /// Details about the step that failed
        when: String,
        /// Underlying error cause
        source: SendError,
    },
    #[snafu(display("error accepting connection: {}", source))]
    /// Error accepting an incoming connection
    Accept {
        /// Underlying error cause
        source: ListenerError,
    },
}
