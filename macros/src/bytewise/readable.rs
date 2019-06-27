// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::configuration::Configuration;

// Functions

pub fn readable(configuration: &Configuration) -> TokenStream {
    let item_ident = configuration.ident();
    let implementation = match configuration {
        Configuration::Struct(item) => {
            let tys = item.marked().map(|acceptor| acceptor.ty());
            let visits = item.marked().map(|acceptor| acceptor.ident()).map(|ident| quote!(visitor.visit(&self.#ident)?;));

            quote! {
                const SIZE: drop::bytewise::Size = <(#(#tys),*) as drop::bytewise::Readable>::SIZE;
                fn accept<Visitor: drop::bytewise::Reader>(&self, visitor: &mut Visitor) -> Result<(), drop::bytewise::ReadError> {
                    #(#visits)*
                    Ok(())
                }
            }
        },
        Configuration::Enum(item) => {
            let arms = item.variants().map(|(discriminant, variant)| {
                let destruct = variant.destruct();
                let visits = variant.marked().map(|acceptor| acceptor.destruct()).map(|acceptor| quote!(visitor.visit(#acceptor)?;));

                quote! {
                    #destruct => {
                        visitor.visit(&#discriminant)?;
                        #(#visits)*
                    }
                }
            });

            quote! {
                const SIZE: drop::bytewise::Size = drop::bytewise::Size::variable();
                fn accept<Visitor: drop::bytewise::Reader>(&self, visitor: &mut Visitor) -> Result<(), drop::bytewise::ReadError> {
                    match self {
                        #(#arms)*
                    }

                    Ok(())
                }
            }
        }
    };

    quote! {
        impl drop::bytewise::Readable for #item_ident {
            #implementation
        }
    }
}
