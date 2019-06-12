// Attributes

#![recursion_limit="128"]

// Crates

extern crate proc_macro;

// Dependencies

use crate::proc_macro::TokenStream;

// Modules

#[cfg_attr(tarpaulin, skip)] mod error;
#[cfg_attr(tarpaulin, skip)] mod typename;

// Procedural macros

#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    error::error(input)
}

#[proc_macro_derive(Typename)]
pub fn typename(input: TokenStream) -> TokenStream {
    typename::typename(input)
}
