// Dependencies

use proc_macro2::TokenStream;
use quote::quote;

// Traits

pub trait Fields {
    fn fields(&self) -> &Vec<Field>;
    fn destruct(&self) -> TokenStream;
}

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

// Implementations

impl Fields for Struct {
    fn fields(&self) -> &Vec<Field> {
        &self.fields
    }

    fn destruct(&self) -> TokenStream {
        destruct(&self.ident, &self.naming, &self.fields)
    }
}

impl Fields for Variant {
    fn fields(&self) -> &Vec<Field> {
        &self.fields
    }

    fn destruct(&self) -> TokenStream {
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
