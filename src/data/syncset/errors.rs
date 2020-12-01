use crate::crypto::hash::HashError;

use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(super)))]
pub enum SyncError {
    #[snafu(display("attempted to hash an empty node"))]
    EmptyHash,
    #[snafu(display("hash collision occured"))]
    Collision,
    #[snafu(display("hash error: {}", source))]
    Hash { source: HashError },
    #[snafu(display("path length error: {}", what))]
    PathLength { what: &'static str },
}
