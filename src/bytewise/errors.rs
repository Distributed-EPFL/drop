// Dependencies

use failure::Fail;

// Structs

#[derive(Fail, Debug)]
#[fail(display = "Unexpectedly reached the end of the buffer.")]
pub struct EndOfBuffer;
