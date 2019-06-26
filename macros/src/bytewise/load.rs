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
        let marked = self.marked().map(|field| {
            let destruct = field.destruct();
            let ty = field.ty();
            quote!(let #destruct = <#ty as drop::bytewise::Load>::load(visitor)?;)
        });

        let unmarked = self.unmarked().map(|field| {
            let destruct = field.destruct();
            quote!(let #destruct = Default::default();)
        });

        let build = self.destruct();

        quote! {{
            #(#marked)*
            #(#unmarked)*
            #build
        }}
    }
}

// Functions

pub fn load(configuration: &Configuration) -> TokenStream {
    let item_ident = configuration.ident();
    let body = match configuration {
        Configuration::Struct(item) => {
            let load = item.load();

            quote! {
                let value = #load;
                Ok(value)
            }
        },
        Configuration::Enum(item) => {
            let arms = item.variants().map(|(discriminant, variant)| {
                let load = variant.load();
                quote!(#discriminant => #load)
            });

            quote! {
                let discriminant = u8::load(visitor)?;
                let value = match discriminant {
                    #(#arms)*,
                    _ => return Err(drop::bytewise::WritableError::new("UnexpectedDiscriminant").into())
                };

                Ok(value)
            }
        }
    };

    quote! {
        impl drop::bytewise::Load for #item_ident {
            fn load<From: drop::bytewise::Writer>(visitor: &mut From) -> Result<Self, drop::bytewise::WriteError> {
                #body
            }
        }
    }
}
