// Dependencies

use super::parse::Error;
use syn::parse_macro_input;
use quote::quote;

// Functions

pub fn error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as Error);

    let output = quote!();
    output.into()
}
