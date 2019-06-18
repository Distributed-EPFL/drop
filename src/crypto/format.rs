// Dependencies

use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;
use super::hash::Digest;
use super::key::Key;

// Implementations

impl Display for Digest {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "<")?;
        for byte in &self.0 { write!(fmt, "{:02x}", byte)?; }
        write!(fmt, ">")?;

        Ok(())
    }
}

impl Debug for Digest {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self)
    }
}

impl Display for Key {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "<")?;
        for byte in &self.0 { write!(fmt, "{:02x}", byte)?; }
        write!(fmt, ">")?;

        Ok(())
    }
}

impl Debug for Key {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self)
    }
}
