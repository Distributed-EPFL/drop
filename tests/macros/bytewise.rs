// Dependencies

use drop::bytewise;
use drop::bytewise::Readable;

// Structs

#[derive(Readable)]
struct Unit;

#[derive(Readable)]
struct UnnamedEmpty();

#[derive(Readable)]
struct UnnamedPartial(u32, #[bytewise] u32, u32, #[bytewise] String);

#[derive(Readable)]
struct Unnamed(#[bytewise] u32, #[bytewise] String);

#[derive(Readable)]
struct NamedEmpty {}

#[derive(Readable)]
#[allow(dead_code)]
struct NamedPartial {
    x: u32,
    #[bytewise] y: u32,
    z: u32,
    #[bytewise] w: String
}

#[derive(Readable)]
struct Named {
    #[bytewise] x: u32,
    #[bytewise] y: String
}

// Test cases

#[test]
fn readable() {
    assert_eq!(bytewise::serialize(&Unit).unwrap(), []);
    assert_eq!(bytewise::serialize(&UnnamedEmpty()).unwrap(), []);
    assert_eq!(bytewise::serialize(&UnnamedPartial(4, 5, 6, "seven".to_string())).unwrap(), [5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    assert_eq!(bytewise::serialize(&Unnamed(5, "seven".to_string())).unwrap(), [5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    assert_eq!(bytewise::serialize(&NamedEmpty{}).unwrap(), []);
    assert_eq!(bytewise::serialize(&NamedPartial{x: 4, y: 5, z: 6, w: "seven".to_string()}).unwrap(), [5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    assert_eq!(bytewise::serialize(&Named{x: 5, y: "seven".to_string()}).unwrap(), [5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
}
