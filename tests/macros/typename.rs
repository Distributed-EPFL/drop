// Dependencies

use drop::traits::Typename;
use macros::Typename;

#[derive(Typename)]
struct MyStruct;

#[derive(Typename)]
struct MyOtherStruct<T>(T);

#[test]
fn develop() {
    println!("{}", MyStruct::typename());
    println!("{}", MyOtherStruct::<MyStruct>::typename());
}
