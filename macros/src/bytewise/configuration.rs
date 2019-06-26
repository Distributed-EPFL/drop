// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use std::iter::Iterator;

// Data structures

pub enum Configuration {
    Struct(Store),
    Enum(Enum)
}

pub struct Store {
    ident: TokenStream,
    naming: Naming,
    fields: Vec<Field>
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

impl Store {
    pub fn new(ident: TokenStream, naming: Naming, fields: Vec<Field>) -> Store {
        Store{ident, naming, fields}
    }

    pub fn ident(&self) -> &TokenStream {
        &self.ident
    }

    pub fn naming(&self) -> &Naming {
        &self.naming
    }

    pub fn fields(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter()
    }

    pub fn marked(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|field| field.marked())
    }

    pub fn unmarked(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|field| !field.marked())
    }

    pub fn destruct(&self) -> TokenStream {
        let ident = &self.ident;
        let fields = self.fields.iter().map(|field| field.destruct());
        match self.naming {
            Naming::Named => quote!(#ident{#(#fields),*}),
            Naming::Unnamed => quote!(#ident(#(#fields),*)),
            Naming::Unit => quote!(#ident)
        }
    }
}

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
