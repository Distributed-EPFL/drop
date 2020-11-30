mod syncset;

// Modules
mod errors;
mod path;
mod set;
mod node;

// Imports
pub use set::Set;
pub use syncset::SyncSet;
pub use path::*;
pub use errors::*;
use node::Node;

// Dependancies
use crate::bytewise::Readable;


// Traits
pub trait Syncable: Clone + Readable + PartialEq {}
impl<T: Clone + Readable + PartialEq> Syncable for T {}

// Constants
const DUMP_THRESHOLD: usize = 5;