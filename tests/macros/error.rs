// Dependencies

use macros::error;

// Enums

#[error]
enum OneOfTwo<Type> {
    Basic(Basic),
    TypeGeneric(TypeGeneric<Type>)
}

#[error]
enum Nested<'s, Type> {
    OneOfTwo(OneOfTwo<Type>),
    LifetimeGeneric(LifetimeGeneric<'s>)
}

#[error]
enum FurtherNested<'s, Type> {
    Nested(Nested<'s, Type>),
    OneOfTwo(OneOfTwo<Type>),
    Generic(Generic<'s, Type>)
}

// Structs

#[error]
struct Basic;

#[error]
struct TypeGeneric<Type>(Type);

#[error]
struct LifetimeGeneric<'s>(&'s u32);

#[error]
struct Generic<'s, Type>(&'s Type);

#[error(x)]
struct ShowOne {
    x: u32
}

#[error(x, y)]
#[derive(Default)]
struct ShowTwo {
    x: u32,
    y: f64,
    #[allow(dead_code)]
    q: String
}

#[error(x, y)]
struct AllTogether<'s, Type> {
    x: u32,
    y: f64,
    #[allow(dead_code)]
    q: &'s Type
}

// Test cases

#[test]
fn struct_display() {
    assert_eq!(format!("{}", Basic), "Basic");
    assert_eq!(format!("{:?}", Basic), "Basic");

    assert_eq!(format!("{}", TypeGeneric(99)), "TypeGeneric");
    assert_eq!(format!("{:?}", TypeGeneric(99)), "TypeGeneric");

    assert_eq!(format!("{}", LifetimeGeneric(&99)), "LifetimeGeneric");
    assert_eq!(format!("{:?}", LifetimeGeneric(&99)), "LifetimeGeneric");

    assert_eq!(format!("{}", Generic(&99)), "Generic");
    assert_eq!(format!("{:?}", Generic(&99)), "Generic");

    assert_eq!(format!("{}", ShowOne{x: 44}), "ShowOne(x: 44)");
    assert_eq!(format!("{:?}", ShowOne{x: 44}), "ShowOne(x: 44)");

    assert_eq!(format!("{}", ShowTwo{x: 44, y: 4.44, ..Default::default()}), "ShowTwo(x: 44, y: 4.44)");
    assert_eq!(format!("{:?}", ShowTwo{x: 44, y: 4.44, ..Default::default()}), "ShowTwo(x: 44, y: 4.44)");

    assert_eq!(format!("{}", AllTogether{x: 44, y: 4.44, q: &444}), "AllTogether(x: 44, y: 4.44)");
    assert_eq!(format!("{:?}", AllTogether{x: 44, y: 4.44, q: &444}), "AllTogether(x: 44, y: 4.44)");
}

#[test]
fn enum_display() {
    assert_eq!(format!("{}", OneOfTwo::<u32>::Basic(Basic)), "Basic");
    assert_eq!(format!("{:?}", OneOfTwo::<u32>::Basic(Basic)), "OneOfTwo <- Basic");

    assert_eq!(format!("{}", Nested::<u32>::LifetimeGeneric(LifetimeGeneric(&44))), "LifetimeGeneric");
    assert_eq!(format!("{:?}", Nested::<u32>::LifetimeGeneric(LifetimeGeneric(&44))), "Nested <- LifetimeGeneric");

    assert_eq!(format!("{}", FurtherNested::<'static, u32>::Nested(Nested::OneOfTwo(OneOfTwo::Basic(Basic)))), "Basic");
    assert_eq!(format!("{:?}", FurtherNested::<'static, u32>::Nested(Nested::OneOfTwo(OneOfTwo::Basic(Basic)))), "FurtherNested <- Nested <- OneOfTwo <- Basic");    
}
