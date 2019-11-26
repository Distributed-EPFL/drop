use super::parse::{Error, ErrorData};

use proc_macro2::TokenStream;
use quote::quote;
use std::vec::Vec;

pub fn nest(error: &Error) -> TokenStream {
    if let ErrorData::Causes(causes) = &error.data {
        let error_ident = &error.idents.error;
        let cause_ident = &error.idents.cause;

        let impls: Vec<TokenStream> = (&causes.unnamed)
            .into_iter()
            .map(|cause| {
                quote! {
                    impl std::convert::From<#cause> for #error_ident {
                        fn from(from: #cause) -> Self {
                            #error_ident::new(#cause_ident::#cause(from))
                        }
                    }
                }
            })
            .collect();

        quote!(#(#impls)*)
    } else {
        TokenStream::new()
    }
}
