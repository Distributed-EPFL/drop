// Dependencies

use drop::bytewise;
use drop::bytewise::Readable;

// Structs

#[derive(Readable)]
struct MyStruct {
    #[bytewise] x: u32,
    y: u64,
    #[bytewise] z: String
}

// Test cases

#[test]
fn develop() {
    println!("{:?}", bytewise::serialize(&MyStruct{x: 44, y: 99, z: "Hello World!".to_string()}));
}
