// Traits

pub trait Sink {
    type Error;
    fn push(&mut self, chunk: &[u8]) -> Result<(), Self::Error>;
}
