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
use serde::Serialize;

// Traits
pub trait Syncable: Serialize + PartialEq {}
impl<T: Serialize + PartialEq> Syncable for T {}

// Constants
const DUMP_THRESHOLD: usize = 5;
