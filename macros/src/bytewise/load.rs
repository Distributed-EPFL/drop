// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::parse::Configuration;
use super::parse::Naming;

// Functions

pub fn load(configuration: &Configuration) -> TokenStream {
    match configuration {
        Configuration::Struct{ident: item_ident, naming, fields} => {
            let loads = fields.into_iter().map(|field| {
                let destruct = &field.destruct;
                let ty = &field.ty;

                if field.marked {
                    quote!(let #destruct = <#ty as drop::bytewise::Load>::load(from)?;)
                } else {
                    quote!(let #destruct = Default::default();)
                }
            });

            let destructs = fields.into_iter().map(|field| &field.destruct);
            let build = match naming {
                Naming::Named => quote!(#item_ident{#(#destructs),*}),
                Naming::Unnamed => quote!(#item_ident(#(#destructs),*)),
                Naming::Unit => quote!(#item_ident)
            };

            quote! {
                impl drop::bytewise::Load for #item_ident {
                    fn load<From: drop::bytewise::Writer>(from: &mut From) -> Result<Self, drop::bytewise::WriteError> {
                        #(#loads)*
                        Ok(#build)
                    }
                }
            }
        },
        Configuration::Enum{ident: item_ident, variants} => unimplemented!()
    }
}
