// Modules

mod build;
mod data;
mod parse;

// Dependencies

use quote::quote;
use syn::parse_macro_input;

// Functions

pub fn error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let error = parse_macro_input!(input as parse::Error);

    let error_struct = data::error(&error);
    let cause_enum = build::causes(&error);

    let methods = build::methods(&error);
    let implementation = build::implementation(&error);

    let from = build::from(&error);

    let debug = build::debug(&error);
    let display = build::display(&error);

    let output = quote! {
        #error_struct
        #cause_enum
        #methods
        #implementation
        #from
        #display
        #debug
    };

    output.into()
}
