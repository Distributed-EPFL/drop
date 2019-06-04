// Dependencies

use macros::error;

// Structs

#[derive(Debug)]
struct Peculiar;

impl std::fmt::Display for Peculiar {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "Peculiar (display)")
    }
}

#[error(i, k, c)]
struct MyError {
    i: u32,
    j: u32,
    k: String,
    c: Peculiar
}

#[error]
struct Alpha;

#[error]
struct Beta;

#[error]
enum WhatWillItBe {
    Alpha(Alpha),
    Beta(Beta),
    MyError(MyError)
}

#[error]
enum IAmConfused {
    Alpha(Alpha),
    WhatWillItBe(WhatWillItBe)
}

// Functions

fn return_something() -> Result<(), Alpha> {
    Err(Alpha)
}

fn return_something_else() -> Result<(), WhatWillItBe> {
    return_something()?;
    Ok(())
}

fn confusedly_return_something() -> Result<(), IAmConfused> {
    return_something_else()?;
    Ok(())
}

// Tests

#[test]
fn error() {
    println!("{:?}", confusedly_return_something());
}
