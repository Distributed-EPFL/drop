// Traits

pub trait Source {
    type Error;
    fn pop(&mut self, size: usize) -> Result<&[u8], Self::Error>;
}
