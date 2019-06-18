// Dependencies

use crate as drop;
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