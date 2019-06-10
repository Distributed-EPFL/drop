// Features

#![feature(const_fn)]
#![feature(specialization)]

// Modules

pub mod bytewise;
pub mod data;
pub mod error;
pub mod traits;

// Traits

pub use bytewise::Load;
pub use bytewise::Readable;
pub use bytewise::Reader;
pub use bytewise::Writable;
pub use bytewise::Writer;
pub use error::Attachment;
pub use error::Context;
pub use error::Error;
pub use traits::Typename;

// Enums

pub use bytewise::Size;

// Structs

pub use data::Varint;

// External structs

pub use backtrace::Backtrace;

// Functions

pub use bytewise::deserialize;
pub use bytewise::serialize;
