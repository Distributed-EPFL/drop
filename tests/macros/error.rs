// Dependencies

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

// Test cases

#[test]
fn develop() {
    let x = MyError::new(99).add("Not easy to solve.").add("When processing `develop()`.");
    let y: MyOtherError = x.into();

    match y.cause() {
        MyOtherErrorCause::MyError(x) => println!("{}: {:?}", x.x(), x.more())
    }
}
