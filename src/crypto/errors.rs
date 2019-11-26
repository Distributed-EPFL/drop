use std::io::Error as IoError;

use macros::error;

use crate as drop;
use crate::error::Error;

use bincode::ErrorKind as BincodeErrorKind;

pub type BincodeError = Box<BincodeErrorKind>;

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
    causes: (ReadError, BincodeError, SodiumError)
}

error! {
    type: SodiumError,
    description: "Sodium failed",
}

error! {
    type: ReadError,
    description: "Invalid data read",
    causes: (IoError)
}

error! {
    type: EncryptError,
    description: "The object provided was impossible to encrypt.",
    causes: (ReadError, BincodeError)
}

error! {
    type: MissingHeader,
    description: "The stream initialization header was not found."
}

error! {
    type: InvalidHeader,
    description: "The stream initialization header was malformed."
}

error! {
    type: InvalidMac,
    description: "Failed to verify message authentication code."
}

error! {
    type: BrokenStream,
    description: "The `RxStream` is broken (it previously incurred in an error)."
}

error! {
    type: DecryptError,
    description: "The ciphertext provided was impossible to decrypt.",
    causes: (MissingHeader, InvalidHeader, InvalidMac, BrokenStream, BincodeError)
}

error! {
    type: ExchangeError,
    description: "unable to exchange key",
    causes: (SodiumError)
}

error! {
    type: SignError,
    description: "failed to sign the data",
    causes: (BincodeError)
}

error! {
    type: VerifyError,
    description: "unable to verify signature",
    causes: (SodiumError, BincodeError),
}
