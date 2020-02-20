mod directory;
pub use self::directory::*;

use std::io::Error;

use super::{ReceiveError, SendError};

use snafu::Snafu;

#[derive(Debug, Snafu)]
/// Errors encountered by servers
pub enum ServerError {
    #[snafu(display("i/o error when {}: {}", when, source))]
    ServerIo { when: String, source: Error },
    #[snafu(display("error receiving data when {}: {}", when, source))]
    Receive { when: String, source: ReceiveError },
    #[snafu(display("error sending data while {}: {}", when, source))]
    Send { when: String, source: SendError },
}
