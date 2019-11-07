mod syncset;

// Modules
mod errors;
mod node;
mod path;
mod set;

// Imports
pub use errors::*;
use node::Node;
pub use path::*;
pub use set::Set;
pub use syncset::SyncSet;

// Dependancies
use crate::bytewise::Readable;

// Traits
pub trait Syncable: Clone + Readable + PartialEq {}
impl<T: Clone + Readable + PartialEq> Syncable for T {}

// Constants
const DUMP_THRESHOLD: usize = 5;
