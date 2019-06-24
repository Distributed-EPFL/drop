// Modules

mod parse;
mod readable;

// Dependencies

use syn::DeriveInput;
use syn::parse_macro_input;

// Functions

pub fn readable(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let configuration = parse::configuration(&input);
    let response = readable::readable(&configuration).into();
    println!("{}", response);
    response
}
