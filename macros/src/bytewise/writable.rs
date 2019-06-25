// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::parse::Configuration;
use super::parse::Naming;

// Functions

pub fn writable(configuration: &Configuration) -> TokenStream {
    match configuration {
        Configuration::Struct{ident: item_ident, fields, ..} => {
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
        Configuration::Enum{ident: item_ident, variants} => {
            let arms = variants.into_iter().enumerate().map(|(discriminant, variant)| {
                let discriminant = discriminant as u8;
                let variant_ident = &variant.ident;

                let destructs = (&variant.fields).into_iter().map(|field| &field.destruct);
                let destruct = match variant.naming {
                    Naming::Named => quote!(#item_ident::#variant_ident{#(#destructs),*}),
                    Naming::Unnamed => quote!(#item_ident::#variant_ident(#(#destructs),*)),
                    Naming::Unit => quote!(#item_ident::#variant_ident)
                };

                let acceptors = (&variant.fields).into_iter().filter(|field| field.marked);
                let visits = acceptors.map(|acceptor| &acceptor.destruct).map(|ident| quote!(visitor.visit(#ident)?;));

                quote! {
                    (#discriminant, #destruct) => {
                        #(#visits)*
                    }
                }
            });

            quote! {
                impl drop::bytewise::Writable for #item_ident {
                    const SIZE: drop::bytewise::Size = drop::bytewise::Size::variable();
                    fn accept<Visitor: drop::bytewise::Writer>(&mut self, visitor: &mut Visitor) -> Result<(), drop::bytewise::WriteError> {
                        let discriminant = u8::load(visitor)?;
                        match(discriminant, self) {
                            #(#arms,)*
                            _ => unimplemented!()
                        }

                        Ok(())
                    }
                }
            }
        }
    }
}
