#![no_std]

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse::Nothing, parse_macro_input};

#[proc_macro]
pub fn get_macro_magic_root(tokens: TokenStream) -> TokenStream {
    let _ = parse_macro_input!(tokens as Nothing);
    let root = match option_env!("MACRO_MAGIC_ROOT") {
        Some(root) => root,
        None => "::macro_magic",
    };
    quote!(#root).into()
}
