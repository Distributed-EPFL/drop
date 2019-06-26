// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::configuration::Configuration;
use super::configuration::Enum;
use super::configuration::Naming;
use super::configuration::Struct;

// Functions

pub fn readable(configuration: &Configuration) -> TokenStream {
    match configuration {
        Configuration::Struct(Struct{ident: item_ident, fields, ..}) => {
            let acceptors = fields.into_iter().filter(|field| field.marked);
            let visits = acceptors.clone().map(|acceptor| &acceptor.ident).map(|ident| quote!(visitor.visit(&self.#ident)?;));
            let tys = acceptors.map(|acceptor| &acceptor.ty);

            quote! {
                impl drop::bytewise::Readable for #item_ident {
                    const SIZE: drop::bytewise::Size = <(#(#tys),*) as drop::bytewise::Readable>::SIZE;
                    fn accept<Visitor: drop::bytewise::Reader>(&self, visitor: &mut Visitor) -> Result<(), drop::bytewise::ReadError> {
                        #(#visits)*
                        Ok(())
                    }
                }
            }
        },
        Configuration::Enum(Enum{ident: item_ident, variants}) => {
            let arms = variants.into_iter().enumerate().map(|(discriminant, variant)| {
                let discriminant = discriminant as u8;
                let variant_ident = &variant.ident;

                let fields = (&variant.fields).into_iter().map(|field| &field.destruct);
                let destruct = match variant.naming {
                    Naming::Named => quote!(#variant_ident{#(#fields),*}),
                    Naming::Unnamed => quote!(#variant_ident(#(#fields),*)),
                    Naming::Unit => quote!(#variant_ident)
                };

                let acceptors = (&variant.fields).into_iter().filter(|field| field.marked).map(|acceptor| &acceptor.destruct);
                let visits = acceptors.map(|acceptor| quote!(visitor.visit(#acceptor)?;));

                quote! {
                    #destruct => {
                        visitor.visit(&#discriminant)?;
                        #(#visits)*
                    }
                }
            });

            quote! {
                impl drop::bytewise::Readable for #item_ident {
                    const SIZE: drop::bytewise::Size = drop::bytewise::Size::variable();
                    fn accept<Visitor: drop::bytewise::Reader>(&self, visitor: &mut Visitor) -> Result<(), drop::bytewise::ReadError> {
                        match self {
                            #(#arms)*
                        }

                        Ok(())
                    }
                }
            }
        }
    }
}
