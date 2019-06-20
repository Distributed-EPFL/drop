// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;

// Constants

const MARKER: &str = "bytewise";

// Structs

pub enum Configuration {
    Struct {
        ident: TokenStream,
        acceptors: Vec<Acceptor>
    }
}

pub struct Acceptor {
    pub ident: TokenStream,
    pub ty: TokenStream
}

// Functions

pub fn configuration(input: &DeriveInput) -> Configuration {
    let ident = &input.ident;
    match &input.data {
        Data::Struct(data) => Configuration::Struct{ident: quote!(#ident), acceptors: acceptors(&data.fields)},
        _ => unimplemented!()
    }
}

fn acceptors(fields: &Fields) -> Vec<Acceptor> {
    let acceptors: Vec<Option<Acceptor>> = match fields {
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

    acceptors.into_iter().filter(|acceptor| acceptor.is_some()).map(|acceptor| acceptor.unwrap()).collect()
}
