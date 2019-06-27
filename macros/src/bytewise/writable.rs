// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::load::Load;
use super::configuration::Configuration;

// Functions

pub fn writable(configuration: &Configuration) -> TokenStream {
    let item_ident = configuration.ident();
    let body = match configuration {
        Configuration::Struct(item) => {
            let visits = item.marked().map(|acceptor| acceptor.ident()).map(|ident| quote!(visitor.visit(&mut self.#ident)?;));
            let tys = item.marked().map(|acceptor| acceptor.ty());

            quote! {
                const SIZE: drop::bytewise::Size = <(#(#tys),*) as drop::bytewise::Writable>::SIZE;
                fn accept<Visitor: drop::bytewise::Writer>(&mut self, visitor: &mut Visitor) -> Result<(), drop::bytewise::WriteError> {
                    #(#visits)*
                    Ok(())
                }
            }
        },
        Configuration::Enum(item) => {
            let discriminant_arms = item.variants().map(|(discriminant, variant)| {
                let case = variant.case();
                quote!(#case => #discriminant)
            });

            let write_arms = item.variants().map(|(_, variant)| {
                let destruct = variant.destruct();
                let visits = variant.marked().map(|acceptor| acceptor.destruct()).map(|ident| quote!(visitor.visit(#ident)?;));

                quote! {
                    #destruct => {
                        #(#visits)*
                    }
                }
            });

            let load_arms = item.variants().map(|(discriminant, variant)| {
                let load = variant.load();
                quote!(#discriminant => #load)
            });

            quote! {
                const SIZE: drop::bytewise::Size = drop::bytewise::Size::variable();
                fn accept<Visitor: drop::bytewise::Writer>(&mut self, visitor: &mut Visitor) -> Result<(), drop::bytewise::WriteError> {
                    let new_discriminant = u8::load(visitor)?;
                    let old_discriminant = match self {
                        #(#discriminant_arms),*
                    };

                    if old_discriminant == new_discriminant {
                        match self {
                            #(#write_arms),*
                        }
                    } else {
                        *self = match new_discriminant {
                            #(#load_arms),*
                            _ => return Err(drop::bytewise::WritableError::new("UnexpectedDeterminant").into())
                        };
                    }

                    Ok(())
                }
            }
        }
    };

    quote! {
        impl drop::bytewise::Writable for #item_ident {
            #body
        }
    }
}
