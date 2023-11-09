//! Macros for producing errors within the derive macro

macro_rules! invalid_input_kind {
    ($arm:expr) => {{
        return syn::Error::new(
            $arm.span(),
            "`#[derive(Parse)]` is only available on structs",
        )
        .to_compile_error();
    }};
}

// Handle a `syn::Result` inside of a function that returns `proc_macro::TokenStream` by turning it
// into a compile error if it's an error
macro_rules! handle_syn_result {
    (
        @default_impl_from($generics_intro:ident, $ident:ident, $generics_args:ident, $where_clause:ident),
        $result:expr
    ) => {{
        let res: syn::Result<_> = $result;
        match res {
            Ok(value) => value,
            Err(e) => {
                let mut ts = quote! {
                    impl #$generics_intro ::syn::parse::Parse for #$ident #$generics_args #$where_clause {
                        fn parse(input: ::syn::parse::ParseStream) -> ::syn::Result<Self> {
                            unimplemented!("failed to derive `Parse`")
                        }
                    }
                };
                ts.extend(e.to_compile_error());
                return ts;
            }
        }
    }};

    ($result:expr) => {{
        let res: syn::Result<_> = $result;
        match res {
            Err(e) => return e.to_compile_error().into(),
            Ok(value) => value,
        }
    }};
}
