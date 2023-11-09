extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;

use syn::*;

#[proc_macro_attribute]
pub fn clonable(_attrs: TokenStream, item: TokenStream) -> TokenStream {
    let mut item_trait = parse_macro_input!(item as ItemTrait);

    let item_trait_ident = &item_trait.ident;

    let cloneish_paths = &[quote!(Clone), quote!(std::clone::Clone), quote!(::std::clone::Clone)];

    if let Some(path) = item_trait
        .supertraits
        .iter_mut()
        .filter_map(|x| match x {
            TypeParamBound::Trait(y) => Some(y),
            _ => None
        })
        .map(|x| &mut x.path)
        .find(|x| {
            let s = quote!(#x).to_string();
            cloneish_paths.iter().any(|y| y.to_string() == s)
        })
    {
        *path = parse_quote!(dyn_clonable::dyn_clone::DynClone);
    } else {
        panic!("`Clone` must be present in trait supertrait list");
    }

    let (impl_generics, ty_generics, where_clause) = item_trait.generics.split_for_impl();

    (quote! {
        #item_trait
        dyn_clonable::dyn_clone::clone_trait_object!(#impl_generics #item_trait_ident #ty_generics #where_clause);
    })
    .into()
}
