// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::configuration::Configuration;
use super::configuration::Field;
use super::configuration::Variant;

// Functions

pub fn load(configuration: &Configuration) -> TokenStream {
    match configuration {
        Configuration::Struct(item) => {
            let item_ident = &item.ident;
            let loads = loads(&item.fields);
            let build = item.destruct();

            quote! {
                impl drop::bytewise::Load for #item_ident {
                    fn load<From: drop::bytewise::Writer>(visitor: &mut From) -> Result<Self, drop::bytewise::WriteError> {
                        #(#loads)*
                        Ok(#build)
                    }
                }
            }
        },
        Configuration::Enum(item) => {
            let item_ident = &item.ident;
            let arms = (&item.variants).into_iter().enumerate().map(|(discriminant, variant)| {
                let discriminant = discriminant as u8;
                let value = self::variant(variant);

                quote! {
                    #discriminant => #value
                }
            });

            quote! {
                impl drop::bytewise::Load for #item_ident {
                    fn load<From: drop::bytewise::Writer>(visitor: &mut From) -> Result<Self, drop::bytewise::WriteError> {
                        let discriminant = u8::load(visitor)?;
                        let value = match discriminant {
                            #(#arms)*,
                            _ => return Err(drop::bytewise::WritableError::new("UnexpectedDiscriminant").into())
                        };
                        Ok(value)
                    }
                }
            }
        }
    }
}

pub fn variant(variant: &Variant) -> TokenStream {
    let loads = loads(&variant.fields);
    let build = variant.destruct();

    quote! {{
        #(#loads)*
        #build
    }}
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
