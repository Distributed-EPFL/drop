use std::fmt;

use super::errors::BrokenStream;
use super::errors::DecryptError;
use super::errors::EncryptError;
use super::errors::InvalidHeader;
use super::errors::InvalidMac;
use super::errors::MissingHeader;
use super::key::Key;

use bincode::{deserialize, serialize_into};

use serde::{Deserialize, Serialize};

use sodiumoxide::crypto::secretstream::Header;
use sodiumoxide::crypto::secretstream::Pull as SodiumPull;
use sodiumoxide::crypto::secretstream::Push as SodiumPush;
use sodiumoxide::crypto::secretstream::Stream;
use sodiumoxide::crypto::secretstream::Tag;
use sodiumoxide::crypto::secretstream::HEADERBYTES;

enum PushState {
    Setup(Key),
    Run {
        stream: Stream<SodiumPush>,
        buffer: Vec<u8>,
    },
}

enum PullState {
    Setup(Key),
    Run(Stream<SodiumPull>),
    Broken,
}

impl fmt::Debug for PullState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Broken => "broken",
                Self::Run(_) => "initialized",
                Self::Setup(_) => "setting up",
            }
        )
    }
}

/// The sending end of an encrypted channel
pub struct Push {
    state: PushState,
}

impl Push {
    pub fn new(key: Key) -> Self {
        Push {
            state: PushState::Setup(key),
        }
    }

    pub fn encrypt<T>(&mut self, message: &T) -> Result<Vec<u8>, EncryptError>
    where
        T: Serialize,
    {
        let encrypt = |stream: &mut Stream<SodiumPush>,
                       mut buffer: &mut Vec<u8>| {
            serialize_into(&mut buffer, message)?;

            let ciphertext = stream.push(&buffer, None, Tag::Message).unwrap();
            buffer.clear();
            Ok(ciphertext)
        };

        match &mut self.state {
            PushState::Setup(key) => {
                let (mut stream, header) =
                    Stream::init_push(&key.clone().into()).unwrap();
                let mut buffer = Vec::new();

                let mut ciphertext = encrypt(&mut stream, &mut buffer)?;
                ciphertext.extend_from_slice(&header[..]);

                self.state = PushState::Run { stream, buffer };
                Ok(ciphertext)
            }
            PushState::Run {
                ref mut stream,
                ref mut buffer,
            } => encrypt(stream, buffer),
        }
    }
}

/// The receiving end of an encrypted channel
pub struct Pull {
    state: PullState,
    buffer: Vec<u8>,
}

impl Pull {
    /// Create a new `Pull` instance using the specified symmetric `Key`
    pub fn new(key: Key) -> Self {
        Pull {
            state: PullState::Setup(key),
            buffer: Vec::new(),
        }
    }

    /// Decrypts an arbitrary message from a slice of bytes. <br />
    /// This method avoids copying data by using a buffer internal
    /// to this `Pull` instance. The resulting value can therefore not
    /// outlive this `Pull` nor can this `Pull` read another message
    /// until the current one has been dropped.
    pub fn decrypt_ref<'de, T>(
        &'de mut self,
        ciphertext: &[u8],
    ) -> Result<T, DecryptError>
    where
        T: Deserialize<'de>,
    {
        match &mut self.state {
            PullState::Setup(key) => {
                if ciphertext.len() < HEADERBYTES {
                    self.state = PullState::Broken;
                    return Err(MissingHeader::new().into());
                }
                let (ciphertext, header) =
                    ciphertext.split_at(ciphertext.len() - HEADERBYTES);

                let mut stream = Stream::init_pull(
                    &Header::from_slice(header).unwrap(),
                    &key.clone().into(),
                )
                .map_err(|_| {
                    self.state = PullState::Broken;
                    InvalidHeader::new()
                })?;

                stream
                    .pull_to_vec(ciphertext, None, &mut self.buffer)
                    .map_err(|_| {
                        self.state = PullState::Broken;
                        InvalidMac::new()
                    })?;

                self.state = PullState::Run(stream);

                deserialize(&self.buffer).map_err(|e| e.into())
            }
            PullState::Run(ref mut stream) => {
                stream
                    .pull_to_vec(ciphertext, None, &mut self.buffer)
                    .map_err(|_| {
                        self.state = PullState::Broken;
                        InvalidMac::new()
                    })?;

                deserialize(&self.buffer).map_err(|e| e.into())
            }
            PullState::Broken => Err(BrokenStream::new().into()),
        }
    }

    /// Decrypts an arbitrary message from a slice of bytes. <br />
    /// The provided data should have been encrypted with
    /// a matching instance of `Push` using the same `Key`. <br />
    /// If this method fails once, the `Pull` instance will be considered
    /// broken and be unusable from then on.
    pub fn decrypt<T>(&mut self, ciphertext: &[u8]) -> Result<T, DecryptError>
    where
        for<'de> T: Deserialize<'de> + ToOwned<Owned = T>,
    {
        self.decrypt_ref(ciphertext)
            .map(|x: T| x.to_owned())
            .map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_test_stream() -> (Push, Pull) {
        let key = Key::random();

        (Push::new(key.clone()), Pull::new(key))
    }

    #[test]
    fn correct_encrypt_decrypt_ref() {
        let (mut transmitter, mut receiver) = setup_test_stream();

        for message in 0u64..128u64 {
            let ciphertext =
                transmitter.encrypt(&message).expect("failed to encrypt");
            let plaintext = receiver
                .decrypt_ref::<u64>(&ciphertext)
                .expect("failed to decrypt without copy");

            assert_eq!(
                plaintext, message,
                "value is different after decryption"
            );
        }
    }

    #[test]
    fn correct_encrypt_decrypt() {
        let (mut transmitter, mut receiver) = setup_test_stream();

        for message in 0u64..128u64 {
            let ciphertext =
                transmitter.encrypt(&message).expect("failed to encrypt");
            let plaintext = receiver
                .decrypt::<u64>(&ciphertext)
                .expect("failed to decrypt");

            assert_eq!(
                plaintext, message,
                "value is different after decryption"
            );
        }
    }

    #[test]
    fn corrupted_mac() {
        let (mut transmitter, mut receiver) = setup_test_stream();

        let mut ciphertext =
            transmitter.encrypt(&0u32).expect("failed to encrypt");

        let ciphertext = ciphertext
            .iter_mut()
            .map(|x| x.wrapping_add(1))
            .collect::<Vec<_>>();

        receiver
            .decrypt::<u32>(&ciphertext)
            .expect_err("decrypted corrupted message");
    }

    #[test]
    fn empty_message() {
        let (_, mut receiver) = setup_test_stream();

        receiver
            .decrypt::<u32>(&[])
            .expect_err("decrypted empty message");
    }

    #[test]
    fn different_key() {
        let (_, mut receiver) = setup_test_stream();
        let (mut transmitter, _) = setup_test_stream();

        let ciphertext = transmitter.encrypt(&0u32).expect("failed to encrypt");

        receiver
            .decrypt::<u32>(&ciphertext)
            .expect_err("decrypted message with wrong key");
    }

    #[test]
    fn broken_stream() {
        let (mut transmitter, mut receiver) = setup_test_stream();

        let mut ciphertext =
            transmitter.encrypt(&0u32).expect("failed to encrypt");
        *ciphertext.first_mut().expect("empty ciphertext") += 1;

        receiver
            .decrypt::<u32>(&ciphertext)
            .expect_err("decrypted corrupted message");

        if let PullState::Broken = receiver.state {
            let ciphertext =
                transmitter.encrypt(&1u32).expect("failed to encrypt");

            receiver
                .decrypt::<u32>(&ciphertext)
                .expect_err("used broken stream without error");
        } else {
            panic!(
                "invalid receiver state after corrupted message: {:#?}",
                receiver.state
            );
        }
    }
}
