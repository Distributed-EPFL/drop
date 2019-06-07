// Dependencies

use drop::error::Error;
use macros::error;

error! {
    type: MyError,
    description: "An error occurred, where x is {x} and y is {y}.",
    fields: {
        x: u32,
        y: u64
    }
}

#[test]
fn develop() {
    let my_error = MyError::new(4, 5);
    println!("{:?}", my_error);
    println!("{:?}", my_error.backtrace());
}
