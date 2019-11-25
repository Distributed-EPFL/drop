mod data;
mod format;
mod impls;
mod nest;
mod parse;

use quote::quote;
use syn::parse_macro_input;

pub fn error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let error = parse_macro_input!(input as parse::Error);

    let error_struct = data::error(&error);
    let cause_enum = data::cause(&error);

    let methods = impls::methods(&error);
    let error_trait = impls::error(&error);

    let nest = nest::nest(&error);

    let display = format::display(&error);
    let debug = format::debug(&error);

    let output = quote! {
        #error_struct
        #cause_enum

        #methods
        #error_trait

        #nest

        #display
        #debug
    };

    output.into()
}
