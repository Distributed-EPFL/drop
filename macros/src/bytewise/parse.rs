// Dependencies

use proc_macro2::Span;
use quote::quote;
use super::configuration::Configuration;
use super::configuration::Field;
use super::configuration::Naming;
use super::configuration::Variant;
use syn::Data;
use syn::DeriveInput;
use syn::Fields;
use syn::Ident;
use syn::IntSuffix;
use syn::LitInt;

// Constants

const MARKER: &str = "bytewise";
const FIELD_PREFIX: &str = "field_";

// Functions

pub fn configuration(input: &DeriveInput) -> Configuration {
    let input_ident = &input.ident;
    match &input.data {
        Data::Struct(data) => Configuration::Struct{ident: quote!(#input_ident), naming: naming(&data.fields), fields: fields(&data.fields)},
        Data::Enum(data) => {
            let variants: Vec<Variant> = (&data.variants).into_iter().map(|variant| {
                let variant_ident = &variant.ident;
                Variant{ident: quote!(#variant_ident), naming: naming(&variant.fields), fields: fields(&variant.fields)}
            }).collect();

            Configuration::Enum{ident: quote!(#input_ident), variants}
        }
        _ => unimplemented!()
    }
}

fn naming(fields: &Fields) -> Naming {
    match fields {
        Fields::Named(_) => Naming::Named,
        Fields::Unnamed(_) => Naming::Unnamed,
        Fields::Unit => Naming::Unit
    }
}

fn fields(fields: &Fields) -> Vec<Field> {
    match fields {
        Fields::Named(fields) => {
            (&fields.named).into_iter().map(|field| {
                let ident = &field.ident;
                let ty = &field.ty;
                let marked = (&field.attrs).into_iter().any(|attr| attr.path.is_ident(MARKER));
                Field{ident: quote!(#ident), destruct: quote!(#ident), ty: quote!(#ty), marked}
            }).collect()
        },
        Fields::Unnamed(fields) => {
            (&fields.unnamed).into_iter().enumerate().map(|(index, field)| {
                let ident = LitInt::new(index as u64, IntSuffix::None, Span::call_site());
                let destruct = Ident::new(&format!("{}{}", FIELD_PREFIX, index), Span::call_site());
                let ty = &field.ty;
                let marked = (&field.attrs).into_iter().any(|attr| attr.path.is_ident(MARKER));
                Field{ident: quote!(#ident), destruct: quote!(#destruct), ty: quote!(#ty), marked}
            }).collect()
        },
        Fields::Unit => Vec::new()
    }
}
