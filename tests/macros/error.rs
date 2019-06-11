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

error! {
    type: MyNestedError,
    description: "A nested error...! This is bad.",
    causes: (MyError)
}

#[test]
fn develop() {
    let result = Result::<(), MyError>::Err(MyError::new(6, 7));
    let result = result.spot(here!()).add("While running `develop`.").attach(44u32);
    let result = result.spot(here!()).add("Seems difficult to fix!").attach(vec!["Hello".to_string(), "World".to_string()]);
    let result: MyNestedError = result.unwrap_err().into();

    println!("{:?}", result);
}
