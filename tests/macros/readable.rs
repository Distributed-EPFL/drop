// Dependencies

use drop::bytewise;
use drop::bytewise::Readable;

// Structs

#[derive(Readable)]
enum MyEnum {
    First(#[bytewise] u32, #[bytewise] u64),
    Second(#[bytewise] u32),
    Third{#[bytewise] hello: String},
    Fourth
}

// Test cases

#[test]
fn develop() {
    let x = MyEnum::First(44, 99);
    println!("{:?}", bytewise::serialize(&x).unwrap());

    let x = MyEnum::Second(33);
    println!("{:?}", bytewise::serialize(&x).unwrap());

    let x = MyEnum::Third{hello: "World".to_string()};
    println!("{:?}", bytewise::serialize(&x).unwrap());

    let x = MyEnum::Fourth;
    println!("{:?}", bytewise::serialize(&x).unwrap());
}
