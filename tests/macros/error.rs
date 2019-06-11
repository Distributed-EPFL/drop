// Dependencies

use drop::error::Context;
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
    let result = Result::<(), MyError>::Err(MyError::new(6, 7));
    let result = result.spot(here!());
    let result = result.spot(here!());

    for spotting in result.unwrap_err().spottings() {
        println!("{}:{}", spotting.file, spotting.line);
    }
}
