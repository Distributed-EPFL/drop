// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::load::Load;
use super::configuration::Configuration;
use super::configuration::Enum;
use super::configuration::Naming;
use super::configuration::Store;

// Functions

pub fn writable(configuration: &Configuration) -> TokenStream {
    match configuration {
        Configuration::Struct(Store{ident: item_ident, fields, ..}) => {
            let acceptors = fields.into_iter().filter(|field| field.marked);
            let visits = acceptors.clone().map(|acceptor| &acceptor.ident).map(|ident| quote!(visitor.visit(&mut self.#ident)?;));
            let tys = acceptors.map(|acceptor| &acceptor.ty);

            quote! {
                impl drop::bytewise::Writable for #item_ident {
                    const SIZE: drop::bytewise::Size = <(#(#tys),*) as drop::bytewise::Writable>::SIZE;
                    fn accept<Visitor: drop::bytewise::Writer>(&mut self, visitor: &mut Visitor) -> Result<(), drop::bytewise::WriteError> {
                        #(#visits)*
                        Ok(())
                    }
                }
            }
        },
        Configuration::Enum(Enum{ident: item_ident, variants}) => {
            let discriminant_arms = variants.into_iter().enumerate().map(|(discriminant, variant)| {
                let variant_ident = &variant.ident;
                let discriminant = discriminant as u8;

                match variant.naming {
                    Naming::Named => quote!(#variant_ident{..} => #discriminant),
                    Naming::Unnamed => quote!(#variant_ident(..) => #discriminant),
                    Naming::Unit => quote!(#variant_ident => #discriminant)
                }
            });

            let write_arms = variants.into_iter().map(|variant| {
                let variant_ident = &variant.ident;

                let destructs = (&variant.fields).into_iter().map(|field| &field.destruct);
                let destruct = match variant.naming {
                    Naming::Named => quote!(#variant_ident{#(#destructs),*}),
                    Naming::Unnamed => quote!(#variant_ident(#(#destructs),*)),
                    Naming::Unit => quote!(#variant_ident)
                };

                let acceptors = (&variant.fields).into_iter().filter(|field| field.marked);
                let visits = acceptors.map(|acceptor| &acceptor.destruct).map(|ident| quote!(visitor.visit(#ident)?;));

                quote! {
                    #destruct => {
                        #(#visits)*
                    }
                }
            });

            let load_arms = variants.into_iter().enumerate().map(|(discriminant, variant)| {
                let discriminant = discriminant as u8;
                let load = variant.load();
                quote! {
                    #discriminant => #load
                }
            });

            quote! {
                impl drop::bytewise::Writable for #item_ident {
                    const SIZE: drop::bytewise::Size = drop::bytewise::Size::variable();
                    fn accept<Visitor: drop::bytewise::Writer>(&mut self, visitor: &mut Visitor) -> Result<(), drop::bytewise::WriteError> {
                        let discriminant = u8::load(visitor)?;
                        if discriminant == match self {
                            #(#discriminant_arms),*
                        } {
                            match self {
                                #(#write_arms),*
                            }
                        } else {
                            *self = match discriminant {
                                #(#load_arms),*
                                _ => return Err(drop::bytewise::WritableError::new("UnexpectedDeterminant").into())
                            };
                        }

                        Ok(())
                    }
                }
            }
        }
    }
}
