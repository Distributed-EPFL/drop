// Dependencies

use proc_macro2::TokenStream;
use quote::ToTokens;
use quote::quote;
use std::vec::Vec;
use syn::Data;
use syn::DataEnum;
use syn::DeriveInput;
use syn::Fields;
use syn::GenericParam;
use syn::Generics;
use syn::Ident;
use syn::Result;
use syn::Token;
use syn::parse::Parse;
use syn::parse::ParseStream;
use syn::parse_macro_input;
use syn::punctuated::Punctuated;

// Aliases

type Visible = Punctuated<Ident, Token![,]>;

// Enums

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

    let (name, generics, specialization) = info(&input);

    let display = show(&visible, &input, Mode::Display);
    let debug = show(&visible, &input, Mode::Debug);
    let nest = nest(&input);

    let output = quote! {
        #input

        #display
        #debug

        impl #generics std::error::Error for #name #specialization {}

        #nest
    };

    output.into()
}

fn info(input: &DeriveInput) -> (&Ident, &Generics, TokenStream) {
    let name = &input.ident;
    let generics = &input.generics;
    let specialization = specialization(generics);

    (name, generics, specialization)
}

fn specialization(generics: &Generics) -> TokenStream {
    if generics.params.len() > 0 {
        let mut specialization = TokenStream::new();

        for parameter in &generics.params {
            let parameter = match parameter.clone() {
                GenericParam::Type(parameter) => parameter.ident.into_token_stream(),
                GenericParam::Lifetime(parameter) => parameter.lifetime.into_token_stream(),
                GenericParam::Const(parameter) => parameter.ident.into_token_stream()
            };

            specialization = quote! {
                #specialization #parameter,
            }
        }

        quote! { <#specialization> }
    } else { TokenStream::new() }
}

fn show(visible: &Visible, input: &DeriveInput, mode: Mode) -> TokenStream {
    let (name, generics, specialization) = info(input);

    let implementation = match mode {
        Mode::Display => quote! { std::fmt::Display },
        Mode::Debug => quote! { std::fmt::Debug }
    };

    match &input.data {
        Data::Struct(_) => {
            let (format, arguments) = format(visible, mode);

            quote! {
                impl #generics #implementation for #name #specialization {
                    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        write!(formatter, #format, stringify!(#name), #arguments)
                    }
                }
            }
        },
        Data::Enum(data) => {
            let dispatch = dispatch(name, data, mode);

            quote! {
                impl #generics #implementation for #name #specialization {
                    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                        match self {
                            #dispatch
                        }
                    }
                }
            }
        },
        _ => panic!("Attribute `#[error]` can only refer to `struct`s or `enum`s.")
    }
}

fn format(visible: &Visible, mode: Mode) -> (String, TokenStream) {
    let (name, item) = match mode {
        Mode::Display => ("{}".to_string(), "{}: {}".to_string()),
        Mode::Debug => ("{}".to_string(), "{}: {:?}".to_string())
    };

    let format = if visible.is_empty() {
        name
    } else {
        let mut formatters = Vec::new();
        formatters.resize(visible.len(), item);
        name + "(" + &formatters.join(", ") + ")"
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
    let operation = match mode {
        Mode::Display => quote! { variant.fmt(formatter) },
        Mode::Debug => quote! {{ write!(formatter, "{} <- ", stringify!(#name))?; variant.fmt(formatter) }}
    };

    let mut dispatch = TokenStream::new();
    for variant in &data.variants {
        let variant = &variant.ident;

        dispatch = quote! {
            #dispatch
            #name::#variant(variant) => #operation,
        }
    }

    dispatch
}

fn nest(input: &DeriveInput) -> TokenStream {
    if let Data::Enum(data) = &input.data {
        let (name, generics, specialization) = info(&input);

        let mut nest = TokenStream::new();
        for variant in &data.variants {
            let source = if let Fields::Unnamed(fields) = &variant.fields {
                let fields = &fields.unnamed;
                if fields.len() != 1 { panic!("Attribute `#[error]` cannot refer to an `enum` with more than one field per variant."); }
                &fields.first().unwrap().value().ty
            } else { panic!("Attribute `#[error]` cannot refer to an `enum` with named variant fields."); };

            let variant = &variant.ident;

            nest = quote! {
                #nest

                impl #generics From<#source> for #name #specialization {
                    fn from(from: #source) -> Self {
                        #name::#variant(from)
                    }
                }
            }
        }

        nest
    } else {
        TokenStream::new()
    }
}
