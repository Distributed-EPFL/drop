// Dependencies

use proc_macro2::Span;
use quote::quote;
use super::configuration::Configuration;
use super::configuration::Enum;
use super::configuration::Field;
use super::configuration::Naming;
use super::configuration::Store;
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
    let item_ident = &input.ident;
    match &input.data {
        Data::Struct(data) => Configuration::Struct(Store::new(quote!(#item_ident), naming(&data.fields), fields(&data.fields))),
        Data::Enum(data) => {
            let variants: Vec<Store> = data.variants.iter().map(|variant| {
                let variant_ident = &variant.ident;
                Store::new(quote!(#item_ident::#variant_ident), naming(&variant.fields), fields(&variant.fields))
            }).collect();

            Configuration::Enum(Enum::new(quote!(#item_ident), variants))
        }
        Data::Union(_) => panic!("Cannot derive `Readable`, `Writable` or `Load` on `union` types.")
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
            fields.named.iter().map(|field| {
                let ident = &field.ident;
                let ty = &field.ty;
                let marked = field.attrs.iter().any(|attr| attr.path.is_ident(MARKER));

                Field::new(quote!(#ident), quote!(#ident), quote!(#ty), marked)
            }).collect()
        },
        Fields::Unnamed(fields) => {
            fields.unnamed.iter().enumerate().map(|(discriminant, field)| {
                let ident = LitInt::new(discriminant as u64, IntSuffix::None, Span::call_site());
                let destruct = Ident::new(&format!("{}{}", FIELD_PREFIX, discriminant), Span::call_site());
                let ty = &field.ty;
                let marked = field.attrs.iter().any(|attr| attr.path.is_ident(MARKER));

                Field::new(quote!(#ident), quote!(#destruct), quote!(#ty), marked)
            }).collect()
        },
        Fields::Unit => Vec::new()
    }
}
