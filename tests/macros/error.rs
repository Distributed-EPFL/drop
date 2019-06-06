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
    let x = MyOtherError{context: std::vec::Vec::new(), cause: MyOtherErrorCause::MyError(MyError{context: std::vec::Vec::new()})};
}
