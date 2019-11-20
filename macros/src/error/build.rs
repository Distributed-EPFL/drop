// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use std::vec::Vec;
use super::parse::Error;
use super::parse::ErrorData;
use syn::Ident;
use syn::parse_macro_input;

// Functions

pub fn error(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let error = parse_macro_input!(input as Error);

    let data = data(&error);
    let causes = causes(&error);
    let methods = methods(&error);
    let implementation = implementation(&error);
    let from = from(&error);

    let output = quote! {
        #data
        #causes
        #methods
        #implementation
        #from
    };

    output.into()
}

fn idents(error: &Error) -> (Ident, Ident) {
    let error_ident = error.ident.clone();
    let cause_ident = Ident::new(&(error_ident.to_string() + "Cause"), error_ident.span());

    (error_ident, cause_ident)
}

fn data(error: &Error) -> TokenStream {
    let (error_ident, cause_ident) = idents(error);

    let mut struct_fields = quote! {
        description: String,
        more: std::vec::Vec<String>
    };

    match &error.data {
        ErrorData::Fields(error_fields) => {
            let error_fields = &error_fields.named;

            struct_fields = quote! {
                #struct_fields,
                #(#error_fields),*
            }
        },
        ErrorData::Causes(_) => {
            struct_fields = quote! {
                #struct_fields,
                cause: #cause_ident
            }
        },
        ErrorData::None => {}
    }

    quote! {
        struct #error_ident {
            #struct_fields
        }
    }
}

fn causes(error: &Error) -> TokenStream {
    if let ErrorData::Causes(causes) = &error.data {
        let cause_ident = idents(error).1;

        // The reference is repeated because, in `quote!`, every interpolation
        // inside of a repetition must be a distinct variable.
        let variants = &causes.unnamed;
        let causes = &causes.unnamed;

        quote! {
            enum #cause_ident {
                #(#variants(#causes)),*
            }
        }
    } else { TokenStream::new() }
}

fn methods(error: &Error) -> TokenStream {
    let (error_ident, cause_ident) = idents(error);
    let description = &error.description;

    let (types, values) = &match &error.data {
        ErrorData::Fields(fields) => {
            let mut types = Vec::new();
            let mut values = Vec::new();

            for field in &fields.named {
                let ty = &field.ty;
                let value = &field.ident;
                types.push(quote!(#ty));
                values.push(quote!(#value));
            }

            (types, values)
        },
        ErrorData::Causes(_) => (vec![quote!(#cause_ident)], vec![quote!(cause)]),
        ErrorData::None => (Vec::new(), Vec::new())
    };

    // The reason of this redundancy is explained above.
    let getters = values;
    let members = values;

    quote! {
        impl #error_ident {
            pub fn new(#(#values: #types),*) -> #error_ident {
                #error_ident{description: #description.to_string(), more: std::vec::Vec::new(), #(#values),*}
            }

            #(
                pub fn #getters<'s>(&'s self) -> &'s #types {
                    &self.#members
                }
            )*
        }
    }
}

fn implementation(error: &Error) -> TokenStream {
    let error_ident = idents(error).0;

    quote! {
        impl drop::Error for #error_ident {
            fn description(&self) -> &String {
                &self.description
            }

            fn add<Text: std::convert::Into<String>>(self, context: Text) -> Self {
                let mut error = self;
                error.more.push(context.into());
                error
            }

            fn more(&self) -> &Vec<String> {
                &self.more
            }
        }
    }
}

fn from(error: &Error) -> TokenStream {
    if let ErrorData::Causes(causes) = &error.data {
        let (error_ident, cause_ident) = idents(error);

        let mut from = TokenStream::new();
        for cause in &causes.unnamed {
            from = quote! {
                #from
                impl std::convert::From<#cause> for #error_ident {
                    fn from(from: #cause) -> Self {
                        #error_ident::new(#cause_ident::#cause(from))
                    }
                }
            }
        }

        from
    } else { TokenStream::new() }
}
