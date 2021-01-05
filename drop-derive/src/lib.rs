use proc_macro2::TokenStream;

use quote::quote;

#[proc_macro_attribute]
pub fn message(
    _metadata: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let def: TokenStream = input.into();

    let expanded = quote! {
        #[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
        #def
    };

    expanded.into()
}
