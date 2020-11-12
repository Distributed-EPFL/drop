mod directory;
pub use self::directory::*;

use std::io::Error as IoError;

use super::{ReceiveError, SendError};
use crate as drop;
use crate::error::Error;

use macros::error;

error! {
    type: ServerError,
    causes: (IoError, ReceiveError, SendError),
    description: "server failure"
}
