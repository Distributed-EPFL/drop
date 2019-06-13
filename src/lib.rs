// Features

#![feature(const_fn)]
#![feature(specialization)]

// Modules

pub mod bytewise;
pub mod data;
pub mod error;
pub mod lang;

// Traits

pub use bytewise::Load;
pub use bytewise::Readable;
pub use bytewise::Reader;
pub use bytewise::Writable;
pub use bytewise::Writer;
pub use error::Context;
pub use error::Error;
pub use lang::Object;
pub use lang::Typename;

// Enums

pub use bytewise::Size;

// Structs

pub use data::Varint;
pub use error::Spotting;

// External structs

pub use backtrace::Backtrace;

// Functions

pub use bytewise::deserialize;
pub use bytewise::serialize;
