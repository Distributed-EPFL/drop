// Features

#![feature(const_fn)]
#![feature(specialization)]

// Modules

pub mod bytewise;
pub mod data;

// Traits

pub use bytewise::Load;
pub use bytewise::Readable;
pub use bytewise::Reader;
pub use bytewise::Writable;
pub use bytewise::Writer;

// Enums

pub use bytewise::Size;

// Structs

pub use data::Varint;

// Functions

pub use bytewise::deserialize;
pub use bytewise::serialize;
