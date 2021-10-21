use std::fmt;
use std::fmt::Debug;
use std::fmt::Display;

use super::hash::Digest;
use super::key::Key;

impl Display for Digest {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "<")?;
        for byte in self.as_bytes() {
            write!(fmt, "{:02x}", byte)?;
        }
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
        for byte in self.as_ref() {
            write!(fmt, "{:02x}", byte)?;
        }
        write!(fmt, ">")?;

        Ok(())
    }
}

impl Debug for Key {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::convert::TryFrom;

    #[test]
    fn reference() {
        assert_eq!(format!("{}", Digest::try_from("0000000000000000000000000000000000000000000000000000000000000000").unwrap()), "<0000000000000000000000000000000000000000000000000000000000000000>");
        assert_eq!(format!("{}", Digest::try_from("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap()), "<0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef>");
        assert_eq!(format!("{:?}", Digest::try_from("0000000000000000000000000000000000000000000000000000000000000000").unwrap()), "<0000000000000000000000000000000000000000000000000000000000000000>");
        assert_eq!(format!("{:?}", Digest::try_from("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap()), "<0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef>");

        assert_eq!(format!("{}", Key::try_from("0000000000000000000000000000000000000000000000000000000000000000").unwrap()), "<0000000000000000000000000000000000000000000000000000000000000000>");
        assert_eq!(format!("{}", Key::try_from("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap()), "<0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef>");
        assert_eq!(format!("{:?}", Key::try_from("0000000000000000000000000000000000000000000000000000000000000000").unwrap()), "<0000000000000000000000000000000000000000000000000000000000000000>");
        assert_eq!(format!("{:?}", Key::try_from("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef").unwrap()), "<0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef>");
    }
}
