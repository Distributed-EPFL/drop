// Dependencies

use proc_macro2::TokenStream;
use std::iter::Iterator;
use super::store::Store;

// Data structures

pub enum Configuration {
    Struct(Store),
    Enum(Enum)
}

pub struct Enum {
    ident: TokenStream,
    variants: Vec<Store>
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

// Implementations

impl Enum {
    pub fn new(ident: TokenStream, variants: Vec<Store>) -> Enum {
        Enum{ident, variants}
    }

    pub fn ident(&self) -> &TokenStream {
        &self.ident
    }

    pub fn variants(&self) -> impl Iterator<Item = (u8, &Store)> {
        self.variants.iter().enumerate().map(|(discriminant, variant)| (discriminant as u8, variant))
    }
}
