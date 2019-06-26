// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::configuration::Configuration;
use super::configuration::Naming;

// Functions

pub fn readable(configuration: &Configuration) -> TokenStream {
    match configuration {
        Configuration::Struct(item) => {
            let item_ident = item.ident();
            let acceptors = item.marked();
            let visits = item.marked().map(|acceptor| &acceptor.ident).map(|ident| quote!(visitor.visit(&self.#ident)?;));
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
        Configuration::Enum(item) => {
            let item_ident = item.ident();
            let arms = item.variants().map(|(discriminant, variant)| {
                let variant_ident = variant.ident();

                let fields = variant.fields().map(|field| &field.destruct);
                let destruct = match variant.naming() {
                    Naming::Named => quote!(#variant_ident{#(#fields),*}),
                    Naming::Unnamed => quote!(#variant_ident(#(#fields),*)),
                    Naming::Unit => quote!(#variant_ident)
                };

                let acceptors = variant.fields().filter(|field| field.marked).map(|acceptor| &acceptor.destruct);
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
