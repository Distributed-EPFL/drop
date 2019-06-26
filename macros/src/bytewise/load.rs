// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::configuration::Configuration;
use super::configuration::Field;
use super::configuration::Naming;
use super::configuration::Variant;

// Functions

pub fn load(configuration: &Configuration) -> TokenStream {
    match configuration {
        Configuration::Struct{ident: item_ident, naming, fields} => {
            let loads = loads(fields);
            let build = build(item_ident, naming, fields);

            quote! {
                impl drop::bytewise::Load for #item_ident {
                    fn load<From: drop::bytewise::Writer>(visitor: &mut From) -> Result<Self, drop::bytewise::WriteError> {
                        #(#loads)*
                        Ok(#build)
                    }
                }
            }
        },
        Configuration::Enum{ident: item_ident, variants} => {
            let arms = variants.into_iter().enumerate().map(|(discriminant, variant)| {
                let discriminant = discriminant as u8;
                let body = self::variant(item_ident, variant);

                quote! {
                    #discriminant => {
                        #body
                    }
                }
            });

            quote! {
                impl drop::bytewise::Load for #item_ident {
                    fn load<From: drop::bytewise::Writer>(visitor: &mut From) -> Result<Self, drop::bytewise::WriteError> {
                        let discriminant = u8::load(visitor)?;
                        let item = match discriminant {
                            #(#arms)*,
                            _ => return Err(drop::bytewise::WritableError::new("UnexpectedDiscriminant").into())
                        };
                        Ok(item)
                    }
                }
            }
        }
    }
}

pub fn variant(item_ident: &TokenStream, variant: &Variant) -> TokenStream {
    let variant_ident = &variant.ident;
    let loads = loads(&variant.fields);
    let build = build(&quote!(#item_ident::#variant_ident), &variant.naming, &variant.fields);

    quote! {
        #(#loads)*
        #build
    }
}

fn loads(fields: &Vec<Field>) -> TokenStream {
    fields.into_iter().map(|field| {
        let destruct = &field.destruct;
        let ty = &field.ty;

        if field.marked {
            quote!(let #destruct = <#ty as drop::bytewise::Load>::load(visitor)?;)
        } else {
            quote!(let #destruct = Default::default();)
        }
    }).collect()
}

fn build(ident: &TokenStream, naming: &Naming, fields: &Vec<Field>) -> TokenStream {
    let destructs = fields.into_iter().map(|field| &field.destruct);
    match naming {
        Naming::Named => quote!(#ident{#(#destructs),*}),
        Naming::Unnamed => quote!(#ident(#(#destructs),*)),
        Naming::Unit => quote!(#ident)
    }
}
