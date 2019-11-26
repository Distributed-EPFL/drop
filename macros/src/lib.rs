// Attributes

#![recursion_limit = "128"]

// Crates

extern crate proc_macro;

// Dependencies

use crate::proc_macro::TokenStream;

// Modules

mod error;

// Procedural macros

#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    error::error(input)
}
