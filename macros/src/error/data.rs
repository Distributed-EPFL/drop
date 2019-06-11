// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::parse::Error;
use super::parse::ErrorData;

// Functions

pub fn error(error: &Error) -> TokenStream {
    let error_ident = &error.idents.error;
    let cause_ident = &error.idents.cause;

    let struct_fields = quote! {
        description: String,
        backtrace: drop::Backtrace,
        spottings: std::vec::Vec<drop::error::Spotting>,
        more: std::vec::Vec<String>,
        attachments: std::vec::Vec<Box<dyn drop::error::Attachment>>
    };

    let struct_fields = match &error.data {
        ErrorData::Fields(error_fields) => {
            let error_fields = &error_fields.named;

            quote! {
                #struct_fields,
                #(#error_fields),*
            }
        },
        ErrorData::Causes(_) => {
            quote! {
                #struct_fields,
                cause: #cause_ident
            }
        },
        ErrorData::None => struct_fields
    };

    quote! {
        struct #error_ident {
            #struct_fields
        }
    }
}
