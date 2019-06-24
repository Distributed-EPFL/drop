// Dependencies

use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::quote;
use super::parse::Configuration;
use super::parse::Naming;

// Functions

pub fn readable(configuration: &Configuration) -> TokenStream {
    match configuration {
        Configuration::Struct{ident: item_ident, acceptors} => {
            let visits = acceptors.into_iter().map(|acceptor| &acceptor.ident).map(|ident| quote!(visitor.visit(&self.#ident)?;));
            let tys = acceptors.into_iter().map(|acceptor| &acceptor.ty);

            quote! {
                impl drop::bytewise::Readable for #item_ident {
                    const SIZE: drop::bytewise::Size = <(#(#tys),*)>::SIZE;
                    fn accept<Visitor: drop::bytewise::Reader>(&self, visitor: &mut Visitor) -> Result<(), drop::bytewise::ReadError> {
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
                let prefix = if let Naming::Unnamed = variant.naming { "field_" } else { "" };

                let fields = (&variant.fields).into_iter().map(|field| syn::Ident::new(&format!("{}{}", prefix, field), Span::call_site()));
                let acceptors = (&variant.acceptors).into_iter().map(|acceptor| syn::Ident::new(&format!("{}{}", prefix, acceptor.ident), Span::call_site()));

                let destruct = match variant.naming {
                    Naming::Named => quote!(#item_ident::#variant_ident{#(#fields),*}),
                    Naming::Unnamed => quote!(#item_ident::#variant_ident(#(#fields),*)),
                    Naming::Unit => quote!(#item_ident::#variant_ident)
                };

                let visits = acceptors.into_iter().map(|acceptor| quote!(visitor.visit(#acceptor)?;));

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
