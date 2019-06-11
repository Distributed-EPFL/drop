// Modules

mod build;
mod parse;

// Dependencies

use quote::quote;
use syn::parse_macro_input;

// Functions

pub fn error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let error = parse_macro_input!(input as parse::Error);

    let data = build::data(&error);
    let causes = build::causes(&error);
    let methods = build::methods(&error);
    let implementation = build::implementation(&error);
    let from = build::from(&error);
    let debug = build::debug(&error);
    let display = build::display(&error);

    let output = quote! {
        #data
        #causes
        #methods
        #implementation
        #from
        #display
        #debug
    };

    output.into()
}
