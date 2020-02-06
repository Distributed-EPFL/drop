use std::fmt;

use super::key::Key;
use super::BincodeError;

use bincode::{deserialize, serialize_into};

use serde::{Deserialize, Serialize};

use snafu::{ensure, Backtrace, ResultExt, Snafu};

use sodiumoxide::crypto::secretstream::{
    Header, Pull as SodiumPull, Push as SodiumPush, Stream, Tag, HEADERBYTES,
};

#[derive(Debug, Snafu)]
pub enum DecryptError {
    #[snafu(display("missing cryptographic header"))]
    MissingHeader { backtrace: Backtrace },

    #[snafu(display("invalid cryptographic header"))]
    InvalidHeader { backtrace: Backtrace },

    #[snafu(display("failed to verify message authentication code"))]
    InvalidMac { backtrace: Backtrace },

    #[snafu(display("stream is broken, an error previously occurred"))]
    BrokenStream { backtrace: Backtrace },

    #[snafu(display("serialization failure: {}", source))]
    SerializeDecrypt { source: BincodeError },
}

#[derive(Debug, Snafu)]
pub enum EncryptError {
    #[snafu(display("failed to serialize: {}", source))]
    SerializeEncrypt { source: BincodeError },
}

enum PushState {
    Setup(Key),
    Run { stream: Stream<SodiumPush> },
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
                Self::Setup(_) => "setting up",
                Self::Run(_) => "initialized",
                Self::Broken => "broken",
            }
        )
    }
}

/// The sending end of an encrypted channel
pub struct Push {
    state: PushState,
    buffer: Vec<u8>,
}

impl Push {
    /// Create a new `Push` using the specified `Key` to encrypt messages
    pub fn new(key: Key) -> Self {
        Push {
            state: PushState::Setup(key),
            buffer: Vec::new(),
        }
    }

    /// Encrypt an arbitrary message into a slice of bytes. <br />
    /// The resulting slice of bytes is allocated and returned as a `Vec<u8>`
    pub fn encrypt<T>(&mut self, message: &T) -> Result<Vec<u8>, EncryptError>
    where
        T: Serialize,
    {
        let encrypt = |stream: &mut Stream<SodiumPush>,
                       mut buffer: &mut Vec<u8>| {
            serialize_into(&mut buffer, message).context(SerializeEncrypt)?;

            let ciphertext = stream.push(&buffer, None, Tag::Message).unwrap();
            buffer.clear();
            Ok(ciphertext)
        };

        match &mut self.state {
            PushState::Setup(key) => {
                let (mut stream, header) =
                    Stream::init_push(&key.clone().into()).unwrap();

                let mut ciphertext = encrypt(&mut stream, &mut self.buffer)?;
                ciphertext.extend_from_slice(&header[..]);

                self.state = PushState::Run { stream };
                Ok(ciphertext)
            }
            PushState::Run { ref mut stream } => {
                encrypt(stream, &mut self.buffer)
            }
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
    pub fn decrypt<'de, T>(
        &'de mut self,
        ciphertext: &[u8],
    ) -> Result<T, DecryptError>
    where
        T: Deserialize<'de>,
    {
        match &mut self.state {
            PullState::Setup(key) => {
                ensure!(ciphertext.len() >= HEADERBYTES, MissingHeader);

                let (ciphertext, header) =
                    ciphertext.split_at(ciphertext.len() - HEADERBYTES);

                let mut stream = Stream::init_pull(
                    &Header::from_slice(header).unwrap(),
                    &key.clone().into(),
                )
                .map_err(|_| {
                    self.state = PullState::Broken;
                    InvalidHeader.build()
                })?;

                stream
                    .pull_to_vec(ciphertext, None, &mut self.buffer)
                    .map_err(|_| {
                        self.state = PullState::Broken;
                        InvalidMac.build()
                    })?;

                self.state = PullState::Run(stream);

                deserialize(&self.buffer).context(SerializeDecrypt)
            }
            PullState::Run(ref mut stream) => {
                stream
                    .pull_to_vec(ciphertext, None, &mut self.buffer)
                    .map_err(|_| {
                        self.state = PullState::Broken;
                        InvalidMac.build()
                    })?;

                deserialize(&self.buffer).context(SerializeDecrypt)
            }
            PullState::Broken => BrokenStream.fail(),
        }
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
                .decrypt::<u64>(&ciphertext)
                .expect("failed to decrypt without copy");

            assert_eq!(plaintext, message, "wrong value decrypted");
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

            assert_eq!(plaintext, message, "wrong value decrypted");
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

    #[test]
    fn garbage_header() {
        let (mut sender, mut receiver) = setup_test_stream();

        let mut msg = sender.encrypt(&0usize).expect("encrypt failed");
        msg[0] = 0x02;
        msg[1] = 0xFF;

        receiver
            .decrypt::<usize>(msg.as_slice())
            .expect_err("decrypt sucess on bad data");
    }

    #[test]
    fn pull_state_fmt() {
        assert_eq!(
            "setting up",
            format!("{:?}", PullState::Setup(Key::random()))
        );
        assert_eq!("broken", format!("{:?}", PullState::Broken));
    }
}
