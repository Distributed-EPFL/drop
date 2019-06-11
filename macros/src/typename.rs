// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use std::vec::Vec;
use syn::DeriveInput;
use syn::GenericParam;
use syn::Ident;
use syn::TypeParam;
use syn::parse_macro_input;

// Functions

pub fn typename(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let where_clause = &input.generics.where_clause;

    let params: Vec<&TypeParam> = (&input.generics.params).into_iter().map(|param| {
        if let GenericParam::Type(param) = param { param } else { panic!("Macro `#[derive(Typename)]` only supports type generics."); }
    }).collect();

    let types: Vec<&Ident> = (&params).into_iter().map(|param| &param.ident).collect();
    let generics: Vec<TokenStream> = (&params).into_iter().map(|param| {
        let (ident, bounds) = (&param.ident, &param.bounds);
        quote!(#ident: drop::traits::Typename #(+ #bounds)*)
    }).collect();

    let (types, generics) = (&types, &generics);

    let format = if types.len() > 0 {
        quote! {
            format!("{}<{}>", stringify!(#ident), vec![#(#types::typename()),*].join(", "))
        }
    } else { quote!(stringify!(#ident).to_string()) };

    let output = quote! {
        impl<#(#generics),*> drop::traits::Typename for #ident<#(#types),*> #where_clause {
            fn typename() -> String {
                #format
            }
        }
    };

    output.into()
}
