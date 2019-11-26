use super::parse::Error;
use super::parse::ErrorData;

use proc_macro2::TokenStream;
use quote::quote;

pub fn error(error: &Error) -> TokenStream {
    let error_ident = &error.idents.error;
    let cause_ident = &error.idents.cause;

    let struct_fields = quote! {
        description: String,
        backtrace: drop::backtrace::Backtrace,
        spottings: std::vec::Vec<drop::error::Spotting>,
        more: std::vec::Vec<String>,
        attachments: std::vec::Vec<(&'static str, Box<std::any::Any>)>
    };

    let struct_fields = match &error.data {
        ErrorData::Fields(error_fields) => {
            let error_fields = &error_fields.named;

            quote! {
                #struct_fields,
                #(#[allow(dead_code)] #error_fields),*
            }
        }
        ErrorData::Causes(_) => {
            quote! {
                #struct_fields,
                cause: #cause_ident
            }
        }
        ErrorData::None => struct_fields,
    };

    quote! {
        pub struct #error_ident {
            #struct_fields
        }
    }
}

pub fn cause(error: &Error) -> TokenStream {
    if let ErrorData::Causes(causes) = &error.data {
        let cause_ident = &error.idents.cause;

        // The reference is repeated because, in `quote!`, every interpolation
        // inside of a repetition must be a distinct variable.
        let variants = &causes.unnamed;
        let causes = &causes.unnamed;

        quote! {
            pub enum #cause_ident {
                #(#variants(#causes)),*
            }
        }
    } else {
        TokenStream::new()
    }
}
