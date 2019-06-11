// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use regex::Regex;
use std::iter;
use std::vec::Vec;
use super::parse::Error;
use super::parse::ErrorData;
use syn::Ident;

// Functions

fn idents(error: &Error) -> (Ident, Ident) {
    let error_ident = error.idents.error.clone();
    let cause_ident = error.idents.cause.clone();

    (error_ident, cause_ident)
}

pub fn debug(error: &Error) -> TokenStream {
    let (error_ident, cause_ident) = idents(error);

    let fields = Regex::new(r"\{([a-zA-Z][a-zA-Z0-9_]*|_[a-zA-Z0-9_]+)\}").unwrap();
    let description = &error.description.value();

    let arguments: Vec<TokenStream> = fields.captures_iter(description).map(|capture| {
        let capture = Ident::new(&capture[1], error.description.span());
        quote!(self.#capture)
    }).collect();

    let format = fields.replace_all(description, "{}").clone();
    let write_description = quote! {
        write!(fmt, "[{}] ", stringify!(#error_ident))?;
        write!(fmt, #format, #(#arguments),*)?;
        for spotting in self.spottings() {
            write!(fmt, "\n  Spotted: {}, line {}", spotting.file, spotting.line)?;
        }
        for more in self.more() {
            write!(fmt, "\n  More: {}", more)?;
        }
        for attachment in self.attachments() {
            write!(fmt, "\n  Attachment: {}", attachment.typename())?
        }
    };

    let recursion = match &error.data {
        ErrorData::Causes(causes) => {
            let causes = &causes.unnamed;
            let cause_ident = iter::repeat(cause_ident);

            quote! {
                match self.cause() {
                    #(#cause_ident::#causes(cause) => {
                        write!(fmt, "\n")?;
                        cause.fmt(fmt)?;
                    }),*
                }
            }
        },
        _ => TokenStream::new()
    };

    quote! {
        impl std::fmt::Debug for #error_ident {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                #write_description
                #recursion
                Ok(())
            }
        }
    }
}
