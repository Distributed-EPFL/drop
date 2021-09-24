use crate::net::connection::SecureError;

use snafu::Snafu;

use std::io::Error;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
/// Error encountered by [`Listener`]s when accepting incoming [`Connection`]s
///
/// [`Listener`]: self::Listener
/// [`Connection`]: super::Connection
pub enum ListenerError {
    #[snafu(visibility(pub))]
    #[snafu(display("i/o  error: {}", source))]
    /// IO error while accepting connection
    Io {
        /// Underlying error cause
        source: Error,
    },

    #[snafu(visibility(pub))]
    #[snafu(display("no address availalble"))]
    /// This listener doesn't have a known candidate
    NoAddress,

    #[snafu(visibility(pub))]
    #[snafu(display("could not secure connection: {}", source))]
    /// Error during handshake
    Secure {
        /// Underlying error cause
        source: SecureError,
    },

    #[snafu(display("{}", reason))]
    #[snafu(visibility(pub))]
    /// Any other type of error
    Other {
        /// The actual cause of the error
        reason: &'static str,
    },
}
