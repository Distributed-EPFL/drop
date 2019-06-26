// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use std::iter::Iterator;
use super::configuration::Field;
use super::configuration::Naming;

// Structs

pub struct Store {
    ident: TokenStream,
    naming: Naming,
    fields: Vec<Field>
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
        self.fields.iter().filter(|field| field.marked)
    }

    pub fn unmarked(&self) -> impl Iterator<Item = &Field> {
        self.fields.iter().filter(|field| !field.marked)
    }

    pub fn destruct(&self) -> TokenStream {
        destruct(&self.ident, &self.naming, &self.fields)
    }
}

// Functions

fn destruct(ident: &TokenStream, naming: &Naming, fields: &Vec<Field>) -> TokenStream {
    let fields = fields.into_iter().map(|field| &field.destruct);
    match naming {
        Naming::Named => quote!(#ident{#(#fields),*}),
        Naming::Unnamed => quote!(#ident(#(#fields),*)),
        Naming::Unit => quote!(#ident)
    }
}
