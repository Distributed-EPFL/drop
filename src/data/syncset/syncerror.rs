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
    description: "A hash collision has occurred (note that this could mean you tried inserting the same element twice)"
}
error! {
    type: SyncError,
    description: "An error has occurred in the SyncSet",
    causes: (HashError, EmptyHashError, CollisionError)
}