// Dependencies

use crate as drop;
use crate::error::Error;
use macros::error;

// Errors

error! {
    type: ReaderError,
    description: "Reader error: {details}.",
    fields: {
        details: &'static str
    }
}

error! {
    type: ReadableError,
    description: "Readable error: {details}.",
    fields: {
        details: &'static str
    }
}

error! {
    type: ReadError,
    description: "Read error.",
    causes: (ReaderError, ReadableError)
}

error! {
    type: WriterError,
    description: "Writer error: {details}.",
    fields: {
        details: &'static str
    }
}

error! {
    type: WritableError,
    description: "Writable error: {details}.",
    fields: {
        details: &'static str
    }
}

error! {
    type: WriteError,
    description: "Write error.",
    causes: (WriterError, WritableError)
}
