// Dependencies

use proc_macro2::TokenStream;

// Data structures

pub enum Configuration {
    Struct {
        ident: TokenStream,
        naming: Naming,
        fields: Vec<Field>
    },
    Enum {
        ident: TokenStream,
        variants: Vec<Variant>
    }
}

pub struct Variant {
    pub ident: TokenStream,
    pub naming: Naming,
    pub fields: Vec<Field>
}

pub enum Naming {
    Named,
    Unnamed,
    Unit
}

pub struct Field {
    pub ident: TokenStream,
    pub destruct: TokenStream,
    pub ty: TokenStream,
    pub marked: bool
}
