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


// Constants

const DUMP_THRESHOLD: usize = 5;