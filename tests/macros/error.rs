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

// Tests

#[test]
fn error() {
    println!("{}", IAmConfused::WhatWillItBe(WhatWillItBe::MyError(MyError{i: 99, j: 44, k: "Hello World!".to_string(), c: Peculiar})));
    println!("{:?}", IAmConfused::WhatWillItBe(WhatWillItBe::MyError(MyError{i: 99, j: 44, k: "Hello World!".to_string(), c: Peculiar})));
}
