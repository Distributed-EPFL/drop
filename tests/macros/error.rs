// Dependencies

use macros::error;

// Traits

trait Reader {
    type Error;
}

trait Readable {
    type Error;
}

// Errors

#[error]
struct ReaderError<ReaderType: Reader>(ReaderType::Error);

#[error]
struct ReadableError<ReadableType: Readable>(ReadableType::Error);

#[error]
enum ReadError<ReaderType: Reader, ReadableType: Readable> {
    Reader(ReaderError<ReaderType>),
    Readable(ReadableError<ReadableType>)
}

// Structs

struct MyReader;
struct MyReadable;

// Implementations

impl Reader for MyReader {
    type Error = u32;
}

impl Readable for MyReadable {
    type Error = u32;
}

// Functions

fn reader_result(first: bool) -> Result<(), ReaderError<MyReader>> {
    if first { Ok(()) } else { Err(ReaderError(99)) }

}

fn readable_result(second: bool) -> Result<(), ReadableError<MyReadable>> {
    if second { Ok(()) } else { Err(ReadableError(84)) }
}

fn read_result(first: bool, second: bool) -> Result<(), ReadError<MyReader, MyReadable>> {
    reader_result(first)?;
    readable_result(second)?;

    Ok(())
}

// Test cases

#[test]
fn error() {
    println!("{:?}", read_result(true, true));
}
