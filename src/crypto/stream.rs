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
            Ok(stream.push(&buffer, None, Tag::Message).unwrap())
        };

        match &mut self.0 {
            TxState::Setup(key) => {
                let (mut stream, header) = Stream::init_push(&key.clone().into()).unwrap();
                let mut buffer = Vec::new();

                let mut ciphertext = encrypt(&mut stream, &mut buffer)?;
                ciphertext.extend_from_slice(&header[..]);

                self.0 = TxState::Run{stream, buffer};
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
        match &mut self.0 {
            RxState::Setup(key) => {
                if ciphertext.len() >= HEADERBYTES {
                    let (ciphertext, header) = ciphertext.split_at(ciphertext.len() - HEADERBYTES);
                    if let Ok(mut stream) = Stream::init_pull(&Header::from_slice(header).unwrap(), &key.clone().into()) {
                        if let Ok((message, _)) = stream.pull(ciphertext, None) {
                            self.0 = RxState::Run(stream);
                            Ok(bytewise::deserialize::<Message>(&message)?)
                        } else { Err(InvalidMac::new().into()) }
                    } else { Err(InvalidHeader::new().into()) }
                } else { Err(MissingHeader::new().into()) }
            },
            RxState::Run(stream) => {
                if let Ok((message, _)) = stream.pull(ciphertext, None) {
                    Ok(bytewise::deserialize::<Message>(&message)?)
                } else { Err(InvalidMac::new().into()) }
            }
        }
    }
}
