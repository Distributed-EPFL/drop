// Modules

mod impls;
mod parse;

// Dependencies

use syn::DeriveInput;
use syn::parse_macro_input;

// Functions

pub fn readable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let configuration = parse::configuration(&input);
    let response = impls::readable(&configuration).into();
    println!("{}", response);
    response
}
