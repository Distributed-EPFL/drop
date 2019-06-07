// Dependencies

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
    println!("{:?}", MyError::new(4, 5));
}
