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
    },
    Enum {
        ident: TokenStream,
        variants: Vec<Variant>
    }
}

pub struct Variant {
    pub ident: TokenStream,
    pub naming: Naming,
    pub fields: Vec<TokenStream>,
    pub acceptors: Vec<Acceptor>
}

pub enum Naming {
    Named,
    Unnamed,
    Unit
}

pub struct Acceptor {
    pub ident: TokenStream,
    pub ty: TokenStream
}

// Functions

pub fn configuration(input: &DeriveInput) -> Configuration {
    let input_ident = &input.ident;
    match &input.data {
        Data::Struct(data) => Configuration::Struct{ident: quote!(#input_ident), acceptors: acceptors(&data.fields)},
        Data::Enum(data) => {
            let variants: Vec<Variant> = (&data.variants).into_iter().map(|variant| {
                let variant_ident = &variant.ident;
                let naming = match &variant.fields { Fields::Named(_) => Naming::Named, Fields::Unnamed(_) => Naming::Unnamed, Fields::Unit => Naming::Unit };

                let fields: Vec<TokenStream> = match &variant.fields {
                    Fields::Named(fields) => (&fields.named).into_iter().map(|field| {
                        let field = field.ident.as_ref().unwrap();
                        quote!(#field)
                    }).collect(),
                    Fields::Unnamed(fields) => (0..fields.unnamed.len()).map(|index| quote!(field_#index)).collect(),
                    Fields::Unit => Vec::new()
                };

                Variant{ident: quote!(#variant_ident), naming, acceptors: acceptors(&variant.fields), fields}
            }).collect();

            Configuration::Enum{ident: quote!(#input_ident), variants}
        }
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
