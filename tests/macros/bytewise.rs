// Dependencies

use drop::bytewise;
use drop::bytewise::Deserializer;
use drop::bytewise::Load;
use drop::bytewise::Readable;
use drop::bytewise::Writable;
use drop::bytewise::Writer;
use std::fmt::Debug;

// Structs

#[derive(Readable, Writable, Load, Default, Debug, PartialEq)]
struct Unit;

#[derive(Readable, Writable, Load, Default, Debug, PartialEq)]
struct UnnamedEmpty();

#[derive(Readable, Writable, Load, Default, Debug, PartialEq)]
struct UnnamedPartial(u32, #[bytewise] u32, u32, #[bytewise] String);

#[derive(Readable, Writable, Load, Default, Debug, PartialEq)]
struct Unnamed(#[bytewise] u32, #[bytewise] String);

#[derive(Readable, Writable, Load, Default, Debug, PartialEq)]
struct NamedEmpty {}

#[derive(Readable, Writable, Load, Default, Debug, PartialEq)]
struct NamedPartial {
    _x: u32,
    #[bytewise] y: u32,
    _z: u32,
    #[bytewise] w: String
}

#[derive(Readable, Writable, Load, Default, Debug, PartialEq)]
struct Named {
    #[bytewise] x: u32,
    #[bytewise] y: String
}

#[derive(Readable, Writable, Load, Debug, PartialEq)]
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

// Functions

fn write_default<Type: Default + Writable + PartialEq + Debug>(reference: Type, bytes: &[u8]) {
    write_on(reference, Type::default(), bytes);
}

fn write_on<Type: Writable + PartialEq + Debug>(reference: Type, mut item: Type, bytes: &[u8]) {
    let mut deserializer = Deserializer(bytes);
    deserializer.visit(&mut item).unwrap();
    assert_eq!(item, reference);
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
fn writable() {
    write_default(Unit, &[]);
    write_default(UnnamedEmpty(), &[]);
    write_default(UnnamedPartial(0, 5, 0, "seven".to_string()), &[5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    write_default(Unnamed(5, "seven".to_string()), &[5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    write_default(NamedEmpty{}, &[]);
    write_default(NamedPartial{_x: 0, y: 5, _z: 0, w: "seven".to_string()}, &[5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    write_default(Named{x: 5, y: "seven".to_string()}, &[5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);

    write_on(UnnamedPartial(1, 5, 1, "seven".to_string()), UnnamedPartial(1, 4, 1, "six".to_string()), &[5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    write_on(NamedPartial{_x: 1, y: 5, _z: 1, w: "seven".to_string()}, NamedPartial{_x: 1, y: 4, _z: 1, w: "six".to_string()}, &[5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);

    write_on(Enum::Unit, Enum::UnnamedEmpty(), &[0]);
    write_on(Enum::UnnamedEmpty(), Enum::Unit, &[1]);
    write_on(Enum::UnnamedPartial(0, 5, 0, "seven".to_string()), Enum::Unit, &[2, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    write_on(Enum::Unnamed(5, "seven".to_string()), Enum::Unit, &[3, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    write_on(Enum::NamedEmpty{}, Enum::Unit, &[4]);
    write_on(Enum::NamedPartial{_x: 0, y: 5, _z: 0, w: "seven".to_string()}, Enum::Unit, &[5, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    write_on(Enum::Named{x: 5, y: "seven".to_string()}, Enum::Unit, &[6, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);

    write_on(Enum::Unit, Enum::Unit, &[0]);
    write_on(Enum::UnnamedEmpty(), Enum::UnnamedEmpty(), &[1]);
    write_on(Enum::UnnamedPartial(1, 5, 1, "seven".to_string()), Enum::UnnamedPartial(1, 4, 1, "six".to_string()), &[2, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    write_on(Enum::Unnamed(5, "seven".to_string()), Enum::Unnamed(2, "eight".to_string()), &[3, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    write_on(Enum::NamedEmpty{}, Enum::NamedEmpty{}, &[4]);
    write_on(Enum::NamedPartial{_x: 1, y: 5, _z: 1, w: "seven".to_string()}, Enum::NamedPartial{_x: 1, y: 4, _z: 1, w: "six".to_string()}, &[5, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
    write_on(Enum::Named{x: 5, y: "seven".to_string()}, Enum::Named{x: 2, y: "eight".to_string()}, &[6, 5, 0, 0, 0, 5, 115, 101, 118, 101, 110]);
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
