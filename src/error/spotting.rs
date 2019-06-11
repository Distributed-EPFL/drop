// Macros

#[macro_export]
macro_rules! here {
    () => (drop::error::Spotting{file: file!(), line: line!()});
}

// Structs

pub struct Spotting {
    pub file: &'static str,
    pub line: u32
}
