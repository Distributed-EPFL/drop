use crate::crypto::{
    key::exchange::ExchangeError,
    stream::{DecryptError, EncryptError},
};

use snafu::{Backtrace, Snafu};

use std::io::Error as IoError;

/// Type of errors returned when serializing/deserializing
pub type SerializerError = Box<bincode::ErrorKind>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
/// Error encountered when attempting to send data on a `Connection`
pub enum SendError {
    #[snafu(display("could not encrypt data: {}", source))]
    /// Error encrypting data before sending
    Encrypt {
        /// Underlying error cause
        source: EncryptError,
    },

    #[snafu(display("could not serialize data: {}", source))]
    /// Data could not be serialized for sending
    SerializeSend {
        /// Underlying error cause
        source: SerializerError,
    },

    #[snafu(display("i/o error: {}", source))]
    /// OS error encountered when sending
    SendIo {
        /// Underlying error cause
        source: IoError,
    },

    #[snafu(display("connection corrupted"))]
    /// Attempted to send data on a corrupted `Connection`
    CorruptedSend {
        /// Backtrace
        backtrace: Backtrace,
    },

    #[snafu(display("unsecured connection"))]
    /// Attempted to send data on an unsecured `Connection`
    UnsecuredSend {
        /// Underlying error cause
        backtrace: Backtrace,
    },
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
/// Error encountered when attempting to receive data on a `Connection`
pub enum ReceiveError {
    #[snafu(display("could not decrypt data: {}", source))]
    /// Error decrypting received data
    Decrypt {
        /// Underlying error cause
        source: DecryptError,
    },

    #[snafu(display("connection is corrupted"))]
    /// Attempted to read from a corrupted `Connection`
    CorruptedReceive {
        /// Error backtrace
        backtrace: Backtrace,
    },

    #[snafu(display("deserialization error: {}", source))]
    /// Error deserializing received data
    DeserializeReceive {
        /// Underlying error cause
        source: SerializerError,
    },

    #[snafu(display("unsecured connection"))]
    /// Attempting a secure receive on an unsecured `Connection`
    UnsecuredReceive {
        /// Error backtrace
        backtrace: Backtrace,
    },

    #[snafu(display("i/o error: {}", source))]
    /// OS error encountered
    ReceiveIo {
        /// Underlying error cause
        source: IoError,
    },
}

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)))]
/// Error encountered when attempting to secure a `Connection`
pub enum SecureError {
    #[snafu(display("could not exchange keys: {}", source))]
    /// Keys could not be exchanged properly
    Exchange {
        /// Underlying error cause
        source: ExchangeError,
    },

    #[snafu(display("i/o error: {}", source))]
    /// OS error occurred while handshaking
    SecureIo {
        /// Underlying error cause
        source: IoError,
    },

    #[snafu(display("receive error: {}", source))]
    /// Error receiving data during handshake
    SecureReceive {
        /// Underlying error cause
        source: ReceiveError,
    },

    #[snafu(display("send error :{}", source))]
    /// Error sending data during handshake
    SecureSend {
        /// Underlying error cause
        source: SendError,
    },
}
