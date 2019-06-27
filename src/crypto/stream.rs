// Dependencies

use crate::bytewise;
use crate::bytewise::Load;
use crate::bytewise::Readable;
use crate::bytewise::Reader;
use crate::bytewise::Serializer;
use sodiumoxide::crypto::secretstream::HEADERBYTES;
use sodiumoxide::crypto::secretstream::Header;
use sodiumoxide::crypto::secretstream::Pull as SodiumPull;
use sodiumoxide::crypto::secretstream::Push as SodiumPush;
use sodiumoxide::crypto::secretstream::Stream;
use sodiumoxide::crypto::secretstream::Tag;
use super::errors::BrokenStream;
use super::errors::DecryptError;
use super::errors::EncryptError;
use super::errors::InvalidHeader;
use super::errors::InvalidMac;
use super::errors::MissingHeader;
use super::key::Key;

// Enums

enum PushState {
    Setup(Key),
    Run {
        stream: Stream<SodiumPush>,
        buffer: Vec<u8>
    }
}

enum PullState {
    Setup(Key),
    Run(Stream<SodiumPull>),
    Broken
}

// Structs

pub struct Push(PushState);
pub struct Pull(PullState);

// Implementations

impl Push {
    pub fn new(key: Key) -> Self {
        Push(PushState::Setup(key))
    }

    pub fn encrypt<Message: Readable>(&mut self, message: &Message) -> Result<Vec<u8>, EncryptError> {
        let encrypt = |stream: &mut Stream<SodiumPush>, buffer: &mut Vec<u8>| {
            let mut serializer = Serializer(buffer);
            serializer.visit(message)?;

            let ciphertext = stream.push(&buffer, None, Tag::Message).unwrap();
            buffer.clear();
            Ok(ciphertext)
        };

        let state = &mut self.0;
        match state {
            PushState::Setup(key) => {
                let (mut stream, header) = Stream::init_push(&key.clone().into()).unwrap();
                let mut buffer = Vec::new();

                let mut ciphertext = encrypt(&mut stream, &mut buffer)?;
                ciphertext.extend_from_slice(&header[..]);

                *state = PushState::Run{stream, buffer};
                Ok(ciphertext)
            },
            PushState::Run{stream, buffer} => encrypt(stream, buffer)
        }
    }
}

impl Pull {
    pub fn new(key: Key) -> Self {
        Pull(PullState::Setup(key))
    }

    pub fn decrypt<Message: Load>(&mut self, ciphertext: &[u8]) -> Result<Message, DecryptError> {
        (|| {
            let state = &mut self.0;
            match state {
                PullState::Setup(key) => {
                    if ciphertext.len() < HEADERBYTES { return Err(MissingHeader::new().into()); }
                    let (ciphertext, header) = ciphertext.split_at(ciphertext.len() - HEADERBYTES);

                    let mut stream = Stream::init_pull(&Header::from_slice(header).unwrap(), &key.clone().into()).map_err(|_| InvalidHeader::new())?;
                    let message = stream.pull(ciphertext, None).map_err(|_| InvalidMac::new())?.0;

                    *state = PullState::Run(stream);
                    Ok(bytewise::deserialize::<Message>(&message)?)
                },
                PullState::Run(stream) => {
                    let message = stream.pull(ciphertext, None).map_err(|_| InvalidMac::new())?.0;
                    Ok(bytewise::deserialize::<Message>(&message)?)
                },
                PullState::Broken => Err(BrokenStream::new().into())
            }
        })().map_err(|error| {
            self.0 = PullState::Broken;
            error
        })
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
        let mut transmitter = Push::new(key.clone());
        let mut receiver = Pull::new(key);

        for message in 0u64..128u64 {
            let ciphertext = transmitter.encrypt(&message).unwrap();
            let plaintext = receiver.decrypt::<u64>(&ciphertext).unwrap();
            assert_eq!(plaintext, message);
        }
    }

    #[test]
    fn errors() {
        let key = Key::random();
        let wrong_key = Key::random();

        let mut receiver = Pull::new(key.clone());
        receiver.decrypt::<u32>(&[]).unwrap_err();

        let mut transmitter = Push::new(key.clone());
        let mut receiver = Pull::new(key.clone());
        let mut ciphertext = transmitter.encrypt(&0u32).unwrap();
        *ciphertext.last_mut().unwrap() += 1;
        receiver.decrypt::<u32>(&ciphertext).unwrap_err();

        let mut transmitter = Push::new(key.clone());
        let mut receiver = Pull::new(key.clone());
        let mut ciphertext = transmitter.encrypt(&0u32).unwrap();
        *ciphertext.first_mut().unwrap() += 1;
        receiver.decrypt::<u32>(&ciphertext).unwrap_err();

        let mut transmitter = Push::new(key.clone());
        let mut receiver = Pull::new(key.clone());
        let ciphertext = transmitter.encrypt(&0u32).unwrap();
        receiver.decrypt::<u32>(&ciphertext).unwrap();
        let mut ciphertext = transmitter.encrypt(&0u32).unwrap();
        *ciphertext.first_mut().unwrap() += 1;
        receiver.decrypt::<u32>(&ciphertext).unwrap_err();
        *ciphertext.first_mut().unwrap() -= 1;
        receiver.decrypt::<u32>(&ciphertext).unwrap_err();

        let mut transmitter = Push::new(key);
        let mut receiver = Pull::new(wrong_key);
        let ciphertext = transmitter.encrypt(&0u32).unwrap();
        receiver.decrypt::<u32>(&ciphertext).unwrap_err();
    }
}
