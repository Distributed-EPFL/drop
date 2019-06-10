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
    
    if let Some(source) = my_error.source() {
        println!("The error comes from {}:{}", source.file.display(), source.line);
    } else {
        println!("Could not determine the source of the error.");
    }
}
