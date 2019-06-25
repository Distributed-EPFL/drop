// Dependencies

use drop::bytewise;
use drop::bytewise::Load;
use drop::bytewise::Readable;
use drop::bytewise::Writable;

// Structs

#[derive(Readable, Writable, Load, Debug, PartialEq)]
struct Unit;

#[derive(Readable, Writable, Load, Debug, PartialEq)]
struct UnnamedEmpty();

#[derive(Readable, Writable, Load, Debug, PartialEq)]
struct UnnamedPartial(u32, #[bytewise] u32, u32, #[bytewise] String);

#[derive(Readable, Writable, Load, Debug, PartialEq)]
struct Unnamed(#[bytewise] u32, #[bytewise] String);

#[derive(Readable, Writable, Load, Debug, PartialEq)]
struct NamedEmpty {}

#[derive(Readable, Writable, Load, Debug, PartialEq)]
struct NamedPartial {
    _x: u32,
    #[bytewise] y: u32,
    _z: u32,
    #[bytewise] w: String
}

#[derive(Readable, Writable, Load, Debug, PartialEq)]
struct Named {
    #[bytewise] x: u32,
    #[bytewise] y: String
}

#[derive(Readable, Load, Debug, PartialEq)]
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

// Implementations

impl Writable for Enum {
    const SIZE: drop::bytewise::Size = drop::bytewise::Size::fixed(0);
    fn accept<Visitor: drop::bytewise::Writer>(&mut self, _: &mut Visitor) -> Result<(), drop::bytewise::WriteError> {
        Ok(())
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

#[test]
fn load() {
    assert_eq!(Unit, bytewise::deserialize(&[]).unwrap());
    assert_eq!(UnnamedEmpty(), bytewise::deserialize(&[]).unwrap());
    assert_eq!(UnnamedPartial(0, 5, 0, "seven".to_string()), bytewise::deserialize(&[5, 0, 0, 0, 5, 115, 101, 118, 101, 110]).unwrap());
    assert_eq!(Unnamed(5, "seven".to_string()), bytewise::deserialize(&[5, 0, 0, 0, 5, 115, 101, 118, 101, 110]).unwrap());
    assert_eq!(NamedEmpty{}, bytewise::deserialize(&[]).unwrap());
    assert_eq!(NamedPartial{_x: 0, y: 5, _z: 0, w: "seven".to_string()}, bytewise::deserialize(&[5, 0, 0, 0, 5, 115, 101, 118, 101, 110]).unwrap());
    assert_eq!(Named{x: 5, y: "seven".to_string()}, bytewise::deserialize(&[5, 0, 0, 0, 5, 115, 101, 118, 101, 110]).unwrap());

    assert_eq!(Enum::Unit, bytewise::deserialize(&[0]).unwrap());
    assert_eq!(Enum::UnnamedEmpty(), bytewise::deserialize(&[1]).unwrap());
    assert_eq!(Enum::UnnamedPartial(0, 5, 0, "seven".to_string()), bytewise::deserialize(&[2, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]).unwrap());
    assert_eq!(Enum::Unnamed(5, "seven".to_string()), bytewise::deserialize(&[3, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]).unwrap());
    assert_eq!(Enum::NamedEmpty{}, bytewise::deserialize(&[4]).unwrap());
    assert_eq!(Enum::NamedPartial{_x: 0, y: 5, _z: 0, w: "seven".to_string()}, bytewise::deserialize(&[5, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]).unwrap());
    assert_eq!(Enum::Named{x: 5, y: "seven".to_string()}, bytewise::deserialize(&[6, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]).unwrap());
}
