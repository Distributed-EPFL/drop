#![recursion_limit = "128"]

extern crate proc_macro;

use crate::proc_macro::TokenStream;

mod error;

#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    error::error(input)
}
