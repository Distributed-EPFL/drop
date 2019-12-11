use serde::Serialize;

mod syncset;

mod errors;
mod node;
mod path;
mod set;

pub use errors::*;
pub use path::*;
pub use set::Set;
pub use syncset::SyncSet;

pub trait Syncable: Serialize + PartialEq {}
impl<T: Serialize + PartialEq> Syncable for T {}

const DUMP_THRESHOLD: usize = 5;
