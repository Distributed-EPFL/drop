// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use regex::Regex;
use super::parse::Error;
use super::parse::ErrorData;
use syn::Ident;

// Functions

pub fn methods(error: &Error) -> TokenStream {
    let error_ident = &error.idents.error;
    let cause_ident = &error.idents.cause;
    let description = description(error);

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
                #error_ident{description: #description, backtrace: drop::Backtrace::new(), spottings: std::vec::Vec::new(), more: std::vec::Vec::new(), attachments: std::vec::Vec::new(), #(#values),*}
            }

            #(
                pub fn #getters<'s>(&'s self) -> &'s #types {
                    &self.#members
                }
            )*
        }
    }
}

fn description(error: &Error) -> TokenStream {
    let error_ident = &error.idents.error;
    let fields = Regex::new(r"\{([a-zA-Z][a-zA-Z0-9_]*|_[a-zA-Z0-9_]+)\}").unwrap();
    let description = &error.description.value();

    let arguments: Vec<TokenStream> = fields.captures_iter(description).map(|capture| {
        let capture = Ident::new(&capture[1], error.description.span());
        quote!(#capture)
    }).collect();

    let format = format!("[{}] {}", error_ident, fields.replace_all(description, "{}"));
    quote!(format!(#format, #(#arguments),*))
}
