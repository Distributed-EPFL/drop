// Dependencies

use quote::quote;
use std::vec::Vec;
use syn::DeriveInput;
use syn::GenericParam;
use syn::parse_macro_input;

// Functions

pub fn typename(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let ident = &input.ident;
    let where_clause = &input.generics.where_clause;

    let mut types = Vec::new();
    let mut generics = Vec::new();

    for generic in &input.generics.params {
        match generic {
            GenericParam::Type(generic) => {
                let ident = &generic.ident;
                let bounds = &generic.bounds;

                types.push(ident);
                generics.push(quote! {
                    #ident: drop::traits::Typename #(+ #bounds)*
                })
            }
            _ => panic!("Macro `#[derive(Typename)]` only supports type generics.")
        }
    }

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
