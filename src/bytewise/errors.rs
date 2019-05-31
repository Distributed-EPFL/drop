// #[kcov(exclude)]

// Dependencies

use failure::Fail;

// Enums

#[derive(Fail, Debug)]
pub enum DeserializeError {
    #[fail(display = "Unexpectedly reached the end of the buffer.")]
    EndOfBuffer
}
