// Dependencies

use proc_macro2::TokenStream;
use quote::quote;

// Data structures

pub enum Configuration {
    Struct(Struct),
    Enum(Enum)
}

pub struct Struct {
    pub ident: TokenStream,
    pub naming: Naming,
    pub fields: Vec<Field>
}

pub struct Enum {
    pub ident: TokenStream,
    pub variants: Vec<Variant>
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
