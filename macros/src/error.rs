// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use std::vec::Vec;
use syn::Data;
use syn::DataEnum;
use syn::DeriveInput;
use syn::Ident;
use syn::Result;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;

// Aliases

type Visible = Punctuated<Ident, Token![,]>;

// Enum

#[derive(Clone, Copy)]
enum Mode {
    Display,
    Debug
}

// Structs

struct VisibleParseProxy(Visible);

// Implementations

impl Parse for VisibleParseProxy {
    fn parse(stream: ParseStream) -> Result<Self> {
        let visible = stream.parse_terminated(Ident::parse)?;
        Ok(VisibleParseProxy(visible))
    }
}

// Functions

pub fn error(options: proc_macro::TokenStream, input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let VisibleParseProxy(visible) = parse_macro_input!(options as VisibleParseProxy);
    let input = parse_macro_input!(input as DeriveInput);

    let name = &input.ident;
    let display = implement(&visible, &input, Mode::Display);
    let debug = implement(&visible, &input, Mode::Debug);

    let output = quote! {
        #input

        #display
        #debug

        impl std::error::Error for #name {}
    };

    output.into()
}

fn implement(visible: &Visible, input: &DeriveInput, mode: Mode) -> TokenStream {
    let name = &input.ident;

    let implementation = match mode {
        Mode::Display => quote! { std::fmt::Display },
        Mode::Debug => quote! { std::fmt::Debug }
    };

    match &input.data {
        Data::Struct(_) => {
            let (format, arguments) = format(visible, mode);

            quote! {
                impl #implementation for #name {
                    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        write!(formatter, #format, stringify!(#name), #arguments)
                    }
                }
            }
        },
        Data::Enum(data) => {
            let dispatch = dispatch(name, data, mode);

            quote! {
                impl #implementation for #name {
                    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        match self {
                            #dispatch
                        }
                    }
                }
            }
        },
        _ => panic!("Attribute #[error] can only refer to `struct`s or `enum`s.")
    }
}

fn format(visible: &Visible, mode: Mode) -> (String, TokenStream) {
    let (header, item) = match mode {
        Mode::Display => ("{}".to_string(), "{}: {}".to_string()),
        Mode::Debug => ("{}".to_string(), "{}: {:?}".to_string())
    };

    let format = if visible.is_empty() {
        header
    } else {
        let mut formatters = Vec::new();
        formatters.resize(visible.len(), item);
        header + "(" + &formatters.join(", ") + ")"
    };

    let mut arguments = TokenStream::new();
    for item in visible {
        arguments = quote! {
            #arguments stringify!(#item), self.#item,
        }
    }

    (format, arguments)
}

fn dispatch(name: &Ident, data: &DataEnum, mode: Mode) -> TokenStream {
    let mut dispatch = TokenStream::new();
    for variant in &data.variants {
        let variant = &variant.ident;

        let operation = match mode {
            Mode::Display => quote! { variant.fmt(formatter) },
            Mode::Debug => quote! {{ write!(formatter, "{} <- ", stringify!(#name))?; variant.fmt(formatter) }}
        };

        dispatch = quote! {
            #dispatch
            #name::#variant(variant) => #operation,
        }
    }

    dispatch
}
