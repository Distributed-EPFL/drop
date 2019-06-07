// Dependencies

use drop::error::Context;
use drop::error::Error;
use macros::error;

error! {
    type: MyError,
    description: "An error occurred!",
    fields: {
        x: u32
    }
}

error! {
    type: MyOtherError,
    description: "Another error occurred!",
    causes: (MyError)
}

// Functions

fn f() -> Result<(), MyError> {
    Err(MyError::new(44))
}

fn g() -> Result<(), MyOtherError> {
    f().add("When calling `f()`").add("Difficult to solve.")?;
    Ok(())
}

// Test cases

#[test]
fn develop() {
    match g().unwrap_err().cause() {
        MyOtherErrorCause::MyError(err) => println!("{:?}", err.more())
    }
}
