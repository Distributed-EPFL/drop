// Dependencies

use drop::lang::Typename;
use macros::Typename;

#[derive(Typename)]
struct MyStruct;

#[derive(Typename)]
struct MyOtherStruct<T>(T);

#[test]
fn develop() {
    println!("{}", <[MyOtherStruct<MyOtherStruct<(u32, u64, f64, std::collections::BinaryHeap<&'static MyStruct>, std::vec::Vec<String>)>>; 2]>::typename());
}
