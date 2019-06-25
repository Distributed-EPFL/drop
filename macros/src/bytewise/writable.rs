// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::parse::Configuration;

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
        Configuration::Enum{..} => unimplemented!()
    }
}
