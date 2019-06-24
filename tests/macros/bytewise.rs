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
struct NamedPartial {
    _x: u32,
    #[bytewise] y: u32,
    _z: u32,
    #[bytewise] w: String
}

#[derive(Readable)]
struct Named {
    #[bytewise] x: u32,
    #[bytewise] y: String
}

#[derive(Readable)]
enum Enum {
    Unit,
    UnnamedEmpty(),
    UnnamedPartial(u32, #[bytewise] u32, u32, #[bytewise] String),
    Unnamed(#[bytewise] u32, #[bytewise] String),
    NamedEmpty {},
    NamedPartial {
        _x: u32,
        #[bytewise] y: u32,
        _z: u32,
        #[bytewise] w: String
    },
    Named {
        #[bytewise] x: u32,
        #[bytewise] y: String
    }
}

// Test cases

#[test]
fn readable() {
    assert_eq!(bytewise::serialize(&Unit).unwrap(), []);
    assert_eq!(bytewise::serialize(&UnnamedEmpty()).unwrap(), []);
    assert_eq!(bytewise::serialize(&UnnamedPartial(4, 5, 6, "seven".to_string())).unwrap(), [5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    assert_eq!(bytewise::serialize(&Unnamed(5, "seven".to_string())).unwrap(), [5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    assert_eq!(bytewise::serialize(&NamedEmpty{}).unwrap(), []);
    assert_eq!(bytewise::serialize(&NamedPartial{_x: 4, y: 5, _z: 6, w: "seven".to_string()}).unwrap(), [5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    assert_eq!(bytewise::serialize(&Named{x: 5, y: "seven".to_string()}).unwrap(), [5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);

    assert_eq!(bytewise::serialize(&Enum::Unit).unwrap(), [0]);
    assert_eq!(bytewise::serialize(&Enum::UnnamedEmpty()).unwrap(), [1]);
    assert_eq!(bytewise::serialize(&Enum::UnnamedPartial(4, 5, 6, "seven".to_string())).unwrap(), [2, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    assert_eq!(bytewise::serialize(&Enum::Unnamed(5, "seven".to_string())).unwrap(), [3, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    assert_eq!(bytewise::serialize(&Enum::NamedEmpty{}).unwrap(), [4]);
    assert_eq!(bytewise::serialize(&Enum::NamedPartial{_x: 4, y: 5, _z: 6, w: "seven".to_string()}).unwrap(), [5, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    assert_eq!(bytewise::serialize(&Enum::Named{x: 5, y: "seven".to_string()}).unwrap(), [6, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
}
