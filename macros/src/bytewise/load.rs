// Dependencies

use proc_macro2::TokenStream;
use quote::quote;
use super::configuration::Configuration;
use super::configuration::Store;

// Traits

pub trait Load {
    fn load(&self) -> TokenStream;
}

// Implementations

impl Load for Store {
    fn load(&self) -> TokenStream {
        let build = self.destruct();
        let loads = self.fields().into_iter().map(|field| {
            let destruct = field.destruct();
            if field.marked() {
                let ty = field.ty();
                quote!(let #destruct = <#ty as drop::bytewise::Load>::load(visitor)?;)
            } else {
                quote!(let #destruct = Default::default();)
            }
        });

        quote! {{
            #(#loads)*
            #build
        }}
    }
}

// Functions

pub fn load(configuration: &Configuration) -> TokenStream {
    match configuration {
        Configuration::Struct(item) => {
            let item_ident = item.ident();
            let load = item.load();

            quote! {
                impl drop::bytewise::Load for #item_ident {
                    fn load<From: drop::bytewise::Writer>(visitor: &mut From) -> Result<Self, drop::bytewise::WriteError> {
                        let value = #load;
                        Ok(value)
                    }
                }
            }
        },
        Configuration::Enum(item) => {
            let item_ident = item.ident();
            let arms = item.variants().map(|(discriminant, variant)| {
                let load = variant.load();
                quote!(#discriminant => #load)
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
