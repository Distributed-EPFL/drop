// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use std::iter;
use super::parse::Error;
use super::parse::ErrorData;

// Functions

pub fn display(error: &Error) -> TokenStream {
    let error_ident = &error.idents.error;
    let cause_ident = &error.idents.cause;

    let implementation = match &error.data {
        ErrorData::Causes(causes) => {
            let causes = &causes.unnamed;
            let cause_ident = iter::repeat(cause_ident);

            quote! {
                match self.cause() {
                    #(#cause_ident::#causes(cause) => {
                        cause.fmt(fmt)?;
                    }),*
                }
            }
        },
        _ => {
            quote! {
                write!(fmt, "[{}] ", stringify!(#error_ident))?;
                if let Some(spotting) = self.spottings().first() {
                    write!(fmt, "at {}, line {}", spotting.file, spotting.line)?;
                }
            }
        }
    };

    quote! {
        impl std::fmt::Display for #error_ident {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                #implementation
                Ok(())
            }
        }
    }
}

pub fn debug(error: &Error) -> TokenStream {
    let error_ident = &error.idents.error;
    let cause_ident = &error.idents.cause;

    let block = quote! {
        write!(fmt, "{}", self.description())?;
        for spotting in self.spottings() {
            write!(fmt, "\n  Spotted: {}, line {}", spotting.file, spotting.line)?;
        }
        for context in self.more() {
            write!(fmt, "\n  Context: {}", context)?;
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
                #block
                #recursion
                Ok(())
            }
        }
    }
}
