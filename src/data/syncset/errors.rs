use crate as drop;
use crate::crypto::HashError;
use crate::error::Error;

use macros::error;

error! {
    type: EmptyHashError,
    description: "Attempted to hash an empty node"
}

error! {
    type: CollisionError,
    description: "A hash collision has occurred"
}

error! {
    type: SyncError,
    description: "An error has occurred in the SyncSet",
    causes: (HashError, EmptyHashError, CollisionError)
}

error! {
    type: PathLengthError,
    description: "Path Length Error: {what}",
    fields: {
        what: &'static str,
    }
}
