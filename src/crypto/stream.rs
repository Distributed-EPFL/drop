// Dependencies

use crate::bytewise;
use crate::bytewise::Load;
use crate::bytewise::Readable;
use crate::bytewise::Reader;
use crate::bytewise::Serializer;
use sodiumoxide::crypto::secretstream::HEADERBYTES;
use sodiumoxide::crypto::secretstream::Header;
use sodiumoxide::crypto::secretstream::Pull;
use sodiumoxide::crypto::secretstream::Push;
use sodiumoxide::crypto::secretstream::Stream;
use sodiumoxide::crypto::secretstream::Tag;
use super::errors::DecryptError;
use super::errors::EncryptError;
use super::errors::InvalidHeader;
use super::errors::InvalidMac;
use super::errors::MissingHeader;
use super::key::Key;

// Enums

enum TxState {
    Setup(Key),
    Run {
        stream: Stream<Push>,
        buffer: Vec<u8>
    }
}

enum RxState {
    Setup(Key),
    Run(Stream<Pull>)
}

// Structs

pub struct TxStream(TxState);
pub struct RxStream(RxState);

// Implementations

impl TxStream {
    pub fn new(key: Key) -> Self {
        TxStream(TxState::Setup(key))
    }

    pub fn encrypt<Message: Readable>(&mut self, message: &Message) -> Result<Vec<u8>, EncryptError> {
        let encrypt = |stream: &mut Stream<Push>, buffer: &mut Vec<u8>| {
            let mut serializer = Serializer(buffer);
            serializer.visit(message)?;

            let ciphertext = stream.push(&buffer, None, Tag::Message).unwrap();
            buffer.clear();
            Ok(ciphertext)
        };

        let state = &mut self.0;
        match state {
            TxState::Setup(key) => {
                let (mut stream, header) = Stream::init_push(&key.clone().into()).unwrap();
                let mut buffer = Vec::new();

                let mut ciphertext = encrypt(&mut stream, &mut buffer)?;
                ciphertext.extend_from_slice(&header[..]);

                *state = TxState::Run{stream, buffer};
                Ok(ciphertext)
            },
            TxState::Run{stream, buffer} => {
                encrypt(stream, buffer)
            }
        }
    }
}

impl RxStream {
    pub fn new(key: Key) -> Self {
        RxStream(RxState::Setup(key))
    }

    pub fn decrypt<Message: Load>(&mut self, ciphertext: &[u8]) -> Result<Message, DecryptError> {
        let state = &mut self.0;
        match state {
            RxState::Setup(key) => {
                if ciphertext.len() < HEADERBYTES { return Err(MissingHeader::new().into()); }
                let (ciphertext, header) = ciphertext.split_at(ciphertext.len() - HEADERBYTES);

                let mut stream = Stream::init_pull(&Header::from_slice(header).unwrap(), &key.clone().into()).map_err(|_| InvalidHeader::new())?;
                let message = stream.pull(ciphertext, None).map_err(|_| InvalidMac::new())?.0;

                *state = RxState::Run(stream);
                Ok(bytewise::deserialize::<Message>(&message)?)
            },
            RxState::Run(stream) => {
                let message = stream.pull(ciphertext, None).map_err(|_| InvalidMac::new())?.0;
                Ok(bytewise::deserialize::<Message>(&message)?)
            }
        }
    }
}

// Tests

#[cfg(test)]
#[cfg_attr(tarpaulin, skip)]
mod tests {
    use super::*;

    #[test]
    fn endpoints() {
        let key = Key::random();
        let mut transmitter = TxStream::new(key.clone());
        let mut receiver = RxStream::new(key);

        for message in 0u64..128u64 {
            let ciphertext = transmitter.encrypt(&message).unwrap();
            let plaintext = receiver.decrypt::<u64>(&ciphertext).unwrap();
            assert_eq!(plaintext, message);
        }
    }
}
