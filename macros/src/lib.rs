// Crates

extern crate proc_macro;

// Dependencies

use crate::proc_macro::TokenStream;

// Modules

mod error;

// Procedural macros

#[proc_macro_attribute]
pub fn error(options: TokenStream, input: TokenStream) -> TokenStream {
    error::error(options, input)
}
