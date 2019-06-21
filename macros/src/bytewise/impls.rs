// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::parse::Configuration;

// Functions

pub fn readable(configuration: &Configuration) -> TokenStream {
    match configuration {
        Configuration::Struct{ident, acceptors} => {
            let visits = acceptors.into_iter().map(|acceptor| &acceptor.ident).map(|ident| quote!(visitor.visit(&self.#ident)?;));
            let tys = acceptors.into_iter().map(|acceptor| &acceptor.ty);

            quote! {
                impl drop::bytewise::Readable for #ident {
                    const SIZE: drop::bytewise::Size = <(#(#tys),*)>::SIZE;
                    fn accept<Visitor: drop::bytewise::Reader>(&self, visitor: &mut Visitor) -> Result<(), drop::bytewise::ReadError> {
                        #(#visits)*
                        Ok(())
                    }
                }
            }
        },
        Configuration::Enum{..} => unimplemented!()
    }
}
