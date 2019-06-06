// Dependencies

use macros::error;

error! {
    type: MyError,
    description: "An error occurred!",
}

error! {
    type: MyOtherError,
    description: "Another error occurred!",
    causes: (MyError)
}

// Test cases

#[test]
fn develop() {
    let x = MyError::new();
    let y: MyOtherError = x.into();
}
