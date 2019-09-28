// Dependencies

use crate as drop;
use crate::error::Error;
use crate::crypto::HashError; 
use macros::error;

// Errors

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
    description: "The provided vector's length was different than expected, or left()/right() was called on a max-length path"
}