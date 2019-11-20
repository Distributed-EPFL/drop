// Modules

mod load;
mod parse;
mod readable;

// Dependencies

use syn::DeriveInput;
use syn::parse_macro_input;

// Functions

pub fn load(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let configuration = parse::configuration(&input);
    load::load(&configuration).into()
}

pub fn readable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let configuration = parse::configuration(&input);
    readable::readable(&configuration).into()
}
