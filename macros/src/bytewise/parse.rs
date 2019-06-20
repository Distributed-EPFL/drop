// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;

// Constants

const MARKER: &str = "bytewise";

// Structs

pub struct Configuration {
    pub ident: TokenStream,
    pub mode: Mode,
    pub acceptors: Vec<Acceptor>
}

pub enum Mode {
    Struct,
    Enum
}

pub struct Acceptor {
    pub ident: TokenStream,
    pub ty: TokenStream
}

// Functions

pub fn configuration(input: &DeriveInput) -> Configuration {
    let ident = &input.ident;
    let (mode, acceptors): (Mode, Vec<Option<Acceptor>>) = match &input.data {
        Data::Struct(data) => {
            let acceptors = match &data.fields {
                Fields::Named(fields) => {
                    (&fields.named).into_iter().map(|field| {
                        if (&field.attrs).into_iter().any(|attr| attr.path.is_ident(MARKER)) {
                            let ident = &field.ident;
                            let ty = &field.ty;
                            Some(Acceptor{ident: quote!(#ident), ty: quote!(#ty)})
                        } else { None }
                    }).collect()
                },
                Fields::Unnamed(fields) => {
                    (&fields.unnamed).into_iter().enumerate().map(|(index, field)| {
                        if (&field.attrs).into_iter().any(|attr| attr.path.is_ident(MARKER)) {
                            let ty = &field.ty;
                            Some(Acceptor{ident: quote!(#index), ty: quote!(#ty)})
                        } else { None }
                    }).collect()
                },
                Fields::Unit => unimplemented!()
            };
            (Mode::Struct, acceptors)
        },
        _ => unimplemented!()
    };

    let acceptors = acceptors.into_iter().filter(|acceptor| acceptor.is_some()).map(|acceptor| acceptor.unwrap()).collect();
    Configuration{ident: quote!(#ident), mode, acceptors}
}
