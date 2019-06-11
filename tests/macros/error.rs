// Dependencies

use drop::error::Error;
use drop::here;
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
    let my_error = MyError::new(9, 7).spot(here!());
    let my_error = my_error.spot(here!());
    for spotting in my_error.spottings() {
        println!("{}:{}", spotting.file, spotting.line);
    }
}
