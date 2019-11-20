#[macro_export]
macro_rules! here {
    () => {
        drop::error::Spotting {
            file: file!(),
            line: line!(),
        }
    };
}

/// A representation of where an `Error` has been seen
pub struct Spotting {
    pub file: &'static str,
    pub line: u32,
}
