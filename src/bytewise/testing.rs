// Dependencies

use std::fmt::Debug;
use std::vec::Vec;
use super::errors::ReaderError;
use super::errors::WriterError;
use super::load::Load;
use super::readable::Readable;
use super::reader::Reader;
use super::writer::Writer;

#[cfg_attr(tarpaulin, skip)]
pub mod reference {
    use super::*;

    // Structs

    pub struct Buffer(&'static [u8]);

    // Implementations

    impl Buffer {
        pub fn new(buffer: &'static [u8]) -> Self {
            Buffer(buffer)
        }

        pub fn empty(&self) -> bool {
            self.0.len() == 0
        }
    }

    impl Reader for Buffer {
        fn push(&mut self, chunk: &[u8]) -> Result<(), ReaderError> {
            if &self.0[0..chunk.len()] == chunk {
                self.0 = &self.0[chunk.len()..];
                Ok(())
            } else { Err(ReaderError::new("UnexpectedInput")) }
        }
    }

    impl Writer for Buffer {
        fn pop(&mut self, size: usize) -> Result<&[u8], WriterError> {
            if size <= self.0.len() {
                let chunk = &self.0[0..size];
                self.0 = &self.0[size..];
                Ok(chunk)
            } else { Err(WriterError::new("EndOfBuffer")) }
        }
    }

    // Functions

    pub fn read<Value: Readable>(value: &Value, reference: &'static [u8]) {
        let mut buffer = Buffer(reference);
        Readable::accept(value, &mut buffer).unwrap();
        assert!(buffer.empty());
    }

    pub fn load<Value: Load + Debug + PartialEq>(value: &'static [u8], reference: &Value) {
        let mut buffer = Buffer(value);
        let value: Value = Load::load(&mut buffer).unwrap();
        assert_eq!(&value, reference);
        assert!(buffer.empty());
    }

    pub fn all<Value: Readable + Load + Debug + PartialEq>(value: &Value, buffer: &'static [u8]) {
        read(value, buffer);
        load(buffer, value);
    }
}

#[cfg_attr(tarpaulin, skip)]
mod invert {
    use super::*;

    // Structs

    pub struct Buffer {
        cursor: usize,
        bytes: Vec<u8>
    }

    // Implementations

    impl Buffer {
        pub fn new() -> Self {
            Buffer{cursor: 0, bytes: Vec::new()}
        }
    }

    impl Reader for Buffer {
        fn push(&mut self, chunk: &[u8]) -> Result<(), ReaderError> {
            self.bytes.extend_from_slice(chunk);
            Ok(())
        }
    }

    impl Writer for Buffer {
        fn pop(&mut self, size: usize) -> Result<&[u8], WriterError> {
            if self.cursor + size <= self.bytes.len() {
                let result = &self.bytes[self.cursor..(self.cursor + size)];
                self.cursor += size;
                Ok(result)
            } else { Err(WriterError::new("EndOfBuffer")) }
        }
    }

    // Functions

    pub fn invert<Value: Readable + Load + Debug + PartialEq>(reference: &Value) {
        let mut buffer = Buffer::new();
        Reader::visit(&mut buffer, reference).unwrap();
        let value = Value::load(&mut buffer).unwrap();
        assert_eq!(&value, reference);
    }
}
