use std::any::type_name;

use drop::error::error;
use drop::error::Error;
use drop::here;

error! {
    type: NoFields,
    description: "No fields."
}

error! {
    type: WithFields,
    description: "Fields are {x} and {y}.",
    fields: {
        x: u32,
        y: &'static str
    }
}

error! {
    type: Nested,
    description: "Nested error.",
    causes: (NoFields, WithFields)
}

error! {
    type: FurtherNested,
    description: "Further nested error.",
    causes: (Nested, NoFields)
}

#[test]
fn implementation() {
    assert_eq!(*WithFields::new(4, "hello").x(), 4);
    assert_eq!(*WithFields::new(4, "hello").y(), "hello");

    assert_eq!(NoFields::new().description(), "[NoFields] No fields.");
    assert_eq!(WithFields::new(4, "hello").description(), "[WithFields] Fields are 4 and hello.");
    assert_eq!(Nested::new(NestedCause::WithFields(WithFields::new(4, "hello"))).description(), "[Nested] Nested error.");

    let file = file!();
    let line = line!();
    let no_fields = NoFields::new().spot(here!());
    assert_eq!(no_fields.spottings()[0].file, file);
    assert_eq!(no_fields.spottings()[0].line, line + 1);

    let no_fields = no_fields.add("hello").add("world");
    assert_eq!(no_fields.more()[0], "hello");
    assert_eq!(no_fields.more()[1], "world");

    let no_fields = no_fields.attach(vec![1u32, 2u32, 3u32, 4u32]);
    assert_eq!(no_fields.attachments()[0].typename(), "Vec<u32>");
    assert_eq!(*no_fields.attachments()[0].downcast_ref::<Vec<u32>>().unwrap(), vec![1u32, 2u32, 3u32, 4u32]);
}

#[test]
fn nesting() {
    let with_fields = WithFields::new(4, "hello");
    let nested: Nested = with_fields.into();
    let no_fields = NoFields::new().add("wrapped");
    let further_nested: FurtherNested = no_fields.into();

    match nested.cause() {
        NestedCause::WithFields(with_fields) => {
            assert_eq!(*with_fields.x(), 4);
            assert_eq!(*with_fields.y(), "hello");
        },
        _ => panic!("Wrong cause for `Nested` error.")
    }

    match further_nested.cause() {
        FurtherNestedCause::NoFields(no_fields) => assert_eq!(no_fields.more()[0], "wrapped"),
        _ => panic!("Wrong cause for `FurtherNested` error.")
    }
}

#[test]
fn format() {
    let file = file!();
    let line = line!();
    let with_fields = WithFields::new(4, "hello").spot(here!()).add("context").attach(77u32);

    assert_eq!(format!("{}", with_fields), format!("[WithFields] at {}, line {}", file, (line + 1)));
    assert_eq!(format!("{:?}", with_fields), format!("[WithFields] Fields are 4 and hello.\n  Spotted: {}, line {}\n  Context: context\n  Attachment: u32", file, (line + 1)));

    let nested: Nested = NoFields::new().into();
    println!("{}", nested);
    println!("{:?}", nested);

    assert_eq!(format!("{}", nested), "[NoFields]");
    assert_eq!(format!("{:?}", nested), "[Nested] Nested error.\n[NoFields] No fields.");
}
