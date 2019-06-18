// Dependencies

use crate as drop;
use crate::bytewise::ReadError;
use crate::error::Error;
use macros::error;

// Errors

error! {
    type: UnexpectedSize,
    description: "The `str` provided has an unexpected size."
}

error! {
    type: MalformedHex,
    description: "The `str` provided contains non-hex characters."
}

error! {
    type: ParseHexError,
    description: "The `str` provided was impossible to parse as an hexadecimal value.",
    causes: (UnexpectedSize, MalformedHex)
}

error! {
    type: HashError,
    description: "The object provided was impossible to hash.",
    causes: (ReadError)
}

error! {
    type: EncryptError,
    description: "The object provided was impossible to encrypt.",
    causes: (ReadError)
}
