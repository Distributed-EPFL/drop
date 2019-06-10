// Attributes

#![recursion_limit="128"]

// Features

#![feature(proc_macro_span)]

// Crates

extern crate proc_macro;

// Dependencies

use crate::proc_macro::TokenStream;

// Modules

mod error;
mod typename;

// Procedural macros

#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    error::error(input)
}

#[proc_macro_derive(Typename)]
pub fn typename(input: TokenStream) -> TokenStream {
    typename::typename(input)
}
