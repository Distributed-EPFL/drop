// Dependencies

use quote::quote;
use syn::parse_macro_input;
use syn::DeriveInput;
use syn::GenericParam;
use syn::Ident;

// Functions

pub fn typename(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let where_clause = &input.generics.where_clause;

    let params = input.generics.params.iter().map(|param| {
        if let GenericParam::Type(param) = param {
            param
        } else {
            panic!("Macro `#[derive(Typename)]` only supports type generics.");
        }
    });

    let types: &Vec<&Ident> =
        &params.clone().map(|param| &param.ident).collect();
    let generics = params.map(|param| {
        let (ident, bounds) = (&param.ident, &param.bounds);
        quote!(#ident: drop::lang::Typename #(+ #bounds)*)
    });

    let format = if types.len() > 0 {
        quote! {
            format!("{}<{}>", stringify!(#ident), [#(#types::typename()),*].join(", "))
        }
    } else {
        quote!(stringify!(#ident).to_string())
    };

    let output = quote! {
        impl<#(#generics),*> drop::lang::Typename for #ident<#(#types),*> #where_clause {
            fn typename() -> String {
                #format
            }
        }
    };

    output.into()
}
