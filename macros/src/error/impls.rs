// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::parse::Error;
use super::parse::ErrorData;

// Functions

pub fn methods(error: &Error) -> TokenStream {
    let error_ident = &error.idents.error;
    let cause_ident = &error.idents.cause;
    let description = &error.description;

    let (types, values) = &match &error.data {
        ErrorData::Fields(fields) => {
            let types: Vec<TokenStream> = (&fields.named).into_iter().map(|field| &field.ty).map(|ty| quote!(#ty)).collect();
            let values: Vec<TokenStream> = (&fields.named).into_iter().map(|field| &field.ident).map(|value| quote!(#value)).collect();
            (types, values)
        },
        ErrorData::Causes(_) => (vec![quote!(#cause_ident)], vec![quote!(cause)]),
        ErrorData::None => (Vec::new(), Vec::new())
    };

    // The reason of this redundancy is explained in data.rs
    let getters = values;
    let members = values;

    quote! {
        impl #error_ident {
            pub fn new(#(#values: #types),*) -> #error_ident {
                #error_ident{description: #description.to_string(), backtrace: drop::Backtrace::new(), spottings: std::vec::Vec::new(), more: std::vec::Vec::new(), attachments: std::vec::Vec::new(), #(#values),*}
            }

            #(
                pub fn #getters<'s>(&'s self) -> &'s #types {
                    &self.#members
                }
            )*
        }
    }
}
