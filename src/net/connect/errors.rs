/// Tcp `Socket` implementation
use crate::net::connection::SecureError;

use snafu::Snafu;

use std::io::{Error, ErrorKind};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
/// Error encountered by [`Connector`] when attempting to establish a [`Connection`]
///
/// [`Connector`]: self::Connector
/// [`Connection`]: super::Connection
pub enum ConnectError {
    #[snafu(display("i/o error: {}", source))]
    /// OS error when connecting
    Io {
        /// Underlying error cause
        source: Error,
    },
    #[snafu(display("could not secure connection: {}", source))]
    /// Error encountered when attempting to secure an outgoing `Connection`
    Secure {
        /// Underlying error cause
        source: SecureError,
    },
    #[snafu(display("underlying connector error: {}", reason))]
    /// Any other kind of error
    Other {
        /// Details about what failed
        reason: String,
    },
}

impl From<ErrorKind> for ConnectError {
    fn from(kind: ErrorKind) -> Self {
        use snafu::IntoError;

        Io {}.into_error(kind.into())
    }
}
