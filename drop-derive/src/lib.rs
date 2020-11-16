use proc_macro2::TokenStream;

use quote::quote;

use syn::{
    parse_macro_input, parse_quote, DeriveInput, GenericParam, Generics,
};

fn add_trait_bounds(mut generics: Generics) -> Generics {
    for generic in &mut generics.params {
        if let GenericParam::Type(ref mut type_param) = *generic {
            type_param.bounds.push(parse_quote!(Message));
        }
    }

    generics
}

#[proc_macro_attribute]
pub fn message(
    _metadata: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let def: TokenStream = input.clone().into();
    let ast = parse_macro_input!(input as DeriveInput);
    let name = ast.ident;

    let generics = add_trait_bounds(ast.generics);

    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let expanded = quote! {
        #[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
        #def

        impl #impl_generics Message for #name #ty_generics #where_clause {}
    };

    expanded.into()
}
