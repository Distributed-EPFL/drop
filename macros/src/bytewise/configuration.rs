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
    ident: TokenStream,
    destruct: TokenStream,
    ty: TokenStream,
    marked: bool
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

impl Field {
    pub fn new(ident: TokenStream, destruct: TokenStream, ty: TokenStream, marked: bool) -> Field {
        Field{ident, destruct, ty, marked}
    }

    pub fn ident(&self) -> &TokenStream {
        &self.ident
    }

    pub fn destruct(&self) -> &TokenStream {
        &self.destruct
    }

    pub fn ty(&self) -> &TokenStream {
        &self.ty
    }

    pub fn marked(&self) -> bool {
        self.marked
    }
}
