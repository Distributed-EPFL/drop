// Dependencies

use proc_macro2::TokenStream;
use super::store::Store;

// Data structures

pub enum Configuration {
    Struct(Store),
    Enum(Enum)
}

pub struct Enum {
    pub ident: TokenStream,
    pub variants: Vec<Store>
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
