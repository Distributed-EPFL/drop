// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use regex::Regex;
use std::iter;
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
    let debug = debug(&error);
    let display = display(&error);

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

fn idents(error: &Error) -> (Ident, Ident) {
    let error_ident = error.ident.clone();
    let cause_ident = Ident::new(&(error_ident.to_string() + "Cause"), error_ident.span());

    (error_ident, cause_ident)
}

fn data(error: &Error) -> TokenStream {
    let (error_ident, cause_ident) = idents(error);

    let mut struct_fields = quote! {
        description: String,
        backtrace: drop::Backtrace,
        spottings: std::vec::Vec<drop::error::Spotting>,
        more: std::vec::Vec<String>,
        attachments: std::vec::Vec<Box<dyn drop::error::Attachment>>
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
                #error_ident{description: #description.to_string(), backtrace: drop::Backtrace::new(), spottings: std::vec::Vec::new(), more: std::vec::Vec::new(), attachments: std::vec::Vec::new(), #(#values),*}
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

            fn backtrace(&self) -> &drop::Backtrace {
                &self.backtrace
            }

            fn spot(self, spotting: drop::error::Spotting) -> Self {
                let mut error = self;
                error.spottings.push(spotting);
                error
            }

            fn add<Text: std::convert::Into<String>>(self, context: Text) -> Self {
                let mut error = self;
                error.more.push(context.into());
                error
            }

            fn attach<Payload: drop::error::Attachment>(self, attachment: Payload) -> Self {
                let attachment = Box::new(attachment);
                let attachment = Box::<dyn drop::error::Attachment>::from(attachment);

                let mut error = self;
                error.attachments.push(attachment);
                error
            }

            fn spottings(&self) -> &Vec<drop::error::Spotting> {
                &self.spottings
            }

            fn more(&self) -> &Vec<String> {
                &self.more
            }

            fn attachments(&self) -> &Vec<Box<dyn drop::error::Attachment>> {
                &self.attachments
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

fn display(error: &Error) -> TokenStream {
    let (error_ident, cause_ident) = idents(error);

    let implementation = match &error.data {
        ErrorData::Causes(causes) => {
            let causes = &causes.unnamed;
            let cause_ident = iter::repeat(cause_ident);

            quote! {
                match self.cause() {
                    #(#cause_ident::#causes(cause) => {
                        cause.fmt(fmt)?;
                    }),*
                }
            }
        },
        _ => {
            quote! {
                write!(fmt, "[{}] ", stringify!(#error_ident))?;
                if let Some(spotting) = self.spottings().first() {
                    write!(fmt, "at {}, line {}", spotting.file, spotting.line)?;
                }
            }
        }
    };

    quote! {
        impl std::fmt::Display for #error_ident {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                #implementation
                Ok(())
            }
        }
    }
}

fn debug(error: &Error) -> TokenStream {
    let (error_ident, cause_ident) = idents(error);

    let fields = Regex::new(r"\{([a-zA-Z][a-zA-Z0-9_]*|_[a-zA-Z0-9_]+)\}").unwrap();
    let description = &error.description.value();

    let arguments: Vec<TokenStream> = fields.captures_iter(description).map(|capture| {
        let capture = Ident::new(&capture[1], error.description.span());
        quote!(self.#capture)
    }).collect();

    let format = fields.replace_all(description, "{}").clone();
    let write_description = quote! {
        write!(fmt, "[{}] ", stringify!(#error_ident))?;
        write!(fmt, #format, #(#arguments),*)?;
        for spotting in self.spottings() {
            write!(fmt, "\n  Spotted: {}, line {}", spotting.file, spotting.line)?;
        }
        for more in self.more() {
            write!(fmt, "\n  More: {}", more)?;
        }
        for attachment in self.attachments() {
            write!(fmt, "\n  Attachment: {}", attachment.typename())?
        }
    };

    let recursion = match &error.data {
        ErrorData::Causes(causes) => {
            let causes = &causes.unnamed;
            let cause_ident = iter::repeat(cause_ident);

            quote! {
                match self.cause() {
                    #(#cause_ident::#causes(cause) => {
                        write!(fmt, "\n")?;
                        cause.fmt(fmt)?;
                    }),*
                }
            }
        },
        _ => TokenStream::new()
    };

    quote! {
        impl std::fmt::Debug for #error_ident {
            fn fmt(&self, fmt: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
                #write_description
                #recursion
                Ok(())
            }
        }
    }
}
