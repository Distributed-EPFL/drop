// Dependencies

use drop::lang::Typename;

#[derive(Typename)]
struct MyStruct;

#[derive(Typename)]
struct MyOtherStruct<T>(T);

#[test]
fn typename() {
    let typename = std::collections::BinaryHeap::<&'static MyStruct>::typename();
    assert_eq!(typename, "BinaryHeap<&'static MyStruct>");

    let typename = <(u32, u64, f64, std::collections::BinaryHeap<&'static MyStruct>, std::vec::Vec<String>)>::typename();
    assert_eq!(typename, "(u32, u64, f64, BinaryHeap<&'static MyStruct>, Vec<String>)");

    let typename = MyOtherStruct::<(u32, u64, f64, std::collections::BinaryHeap<&'static MyStruct>, std::vec::Vec<String>)>::typename();
    assert_eq!(typename, "MyOtherStruct<(u32, u64, f64, BinaryHeap<&'static MyStruct>, Vec<String>)>");

    let typename = MyOtherStruct::<MyOtherStruct<(u32, u64, f64, std::collections::BinaryHeap<&'static MyStruct>, std::vec::Vec<String>)>>::typename();
    assert_eq!(typename, "MyOtherStruct<MyOtherStruct<(u32, u64, f64, BinaryHeap<&'static MyStruct>, Vec<String>)>>");

    let typename = <[MyOtherStruct<MyOtherStruct<(u32, u64, f64, std::collections::BinaryHeap<&'static MyStruct>, std::vec::Vec<String>)>>; 2]>::typename();
    assert_eq!(typename, "[MyOtherStruct<MyOtherStruct<(u32, u64, f64, BinaryHeap<&'static MyStruct>, Vec<String>)>>; 2]");
}
