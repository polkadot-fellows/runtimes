use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{parse_quote, Data, DataStruct, DeriveInput, Ident, Lifetime, WherePredicate};

#[derive(Clone, Copy, Debug, PartialEq)]
enum Asn1Type {
    Ber,
    Der,
}

pub fn derive_ber_sequence(s: synstructure::Structure) -> proc_macro2::TokenStream {
    let ast = s.ast();

    let container = match &ast.data {
        Data::Struct(ds) => Container::from_datastruct(ds, ast),
        _ => panic!("Unsupported type, cannot derive"),
    };

    let debug_derive = ast.attrs.iter().any(|attr| {
        attr.path
            .is_ident(&Ident::new("debug_derive", Span::call_site()))
    });

    let impl_tryfrom = container.gen_tryfrom();
    let impl_tagged = container.gen_tagged();
    let ts = s.gen_impl(quote! {
        extern crate asn1_rs;

        #impl_tryfrom
        #impl_tagged
    });
    if debug_derive {
        eprintln!("{}", ts.to_string());
    }
    ts
}

pub fn derive_der_sequence(s: synstructure::Structure) -> proc_macro2::TokenStream {
    let ast = s.ast();

    let container = match &ast.data {
        Data::Struct(ds) => Container::from_datastruct(ds, ast),
        _ => panic!("Unsupported type, cannot derive"),
    };

    let debug_derive = ast.attrs.iter().any(|attr| {
        attr.path
            .is_ident(&Ident::new("debug_derive", Span::call_site()))
    });

    let impl_tryfrom = container.gen_tryfrom();
    let impl_tagged = container.gen_tagged();
    // let impl_checkconstraints = container.gen_checkconstraints();
    let impl_fromder = container.gen_fromder();
    let ts = s.gen_impl(quote! {
        extern crate asn1_rs;

        #impl_tryfrom
        #impl_tagged
        //#impl_checkconstraints
        #impl_fromder
    });
    if debug_derive {
        eprintln!("{}", ts.to_string());
    }
    ts
}

pub struct Container {
    field_names: Vec<Ident>,
    where_predicates: Vec<WherePredicate>,
}

impl Container {
    pub fn from_datastruct(ds: &DataStruct, ast: &DeriveInput) -> Self {
        let field_names: Vec<_> = ds.fields.iter().map(|f| f.ident.clone().unwrap()).collect();
        // dbg!(s);

        // get lifetimes from generics
        let lfts: Vec<_> = ast.generics.lifetimes().collect();
        let mut where_predicates = Vec::new();
        if !lfts.is_empty() {
            // input slice must outlive all lifetimes from Self
            let lft = Lifetime::new("'ber", Span::call_site());
            let wh: WherePredicate = parse_quote! { #lft: #(#lfts)+* };
            where_predicates.push(wh);
        };

        Container {
            field_names,
            where_predicates,
        }
    }

    pub fn gen_tryfrom(&self) -> TokenStream {
        let field_names = &self.field_names;
        let parse_content = derive_ber_sequence_content(&field_names, Asn1Type::Ber);
        let lifetime = Lifetime::new("'ber", Span::call_site());
        let wh = &self.where_predicates;
        // note: `gen impl` in synstructure takes care of appending extra where clauses if any, and removing
        // the `where` statement if there are none.
        quote! {
            use asn1_rs::{Any, FromBer};
            use core::convert::TryFrom;

            gen impl<#lifetime> TryFrom<Any<#lifetime>> for @Self where #(#wh)+* {
                type Error = asn1_rs::Error;

                fn try_from(any: Any<#lifetime>) -> asn1_rs::Result<Self> {
                    any.tag().assert_eq(Self::TAG)?;

                    // no need to parse sequence, we already have content
                    let i = any.data;
                    //
                    #parse_content
                    //
                    let _ = i; // XXX check if empty?
                    Ok(Self{#(#field_names),*})
                }
            }
        }
    }

    pub fn gen_tagged(&self) -> TokenStream {
        quote! {
            gen impl<'ber> asn1_rs::Tagged for @Self {
                const TAG: asn1_rs::Tag = asn1_rs::Tag::Sequence;
            }
        }
    }

    #[allow(dead_code)]
    pub fn gen_checkconstraints(&self) -> TokenStream {
        quote! {
            gen impl<'ber> asn1_rs::CheckDerConstraints for @Self {
                fn check_constraints(any: &Any) -> Result<()> {
                    any.tag().assert_eq(Self::TAG)?;
                    Ok(())
                }
            }
        }
    }

    pub fn gen_fromder(&self) -> TokenStream {
        let lifetime = Lifetime::new("'ber", Span::call_site());
        let wh = &self.where_predicates;
        let field_names = &self.field_names;
        let parse_content = derive_ber_sequence_content(&field_names, Asn1Type::Der);
        // note: `gen impl` in synstructure takes care of appending extra where clauses if any, and removing
        // the `where` statement if there are none.
        quote! {
            use asn1_rs::{FromDer, Tagged};

            gen impl<#lifetime> asn1_rs::FromDer<#lifetime> for @Self where #(#wh)+* {
                fn from_der(bytes: &#lifetime [u8]) -> asn1_rs::ParseResult<#lifetime, Self> {
                    let (rem, any) = asn1_rs::Any::from_der(bytes)?;
                    any.header.assert_tag(Self::TAG)?;
                    let i = any.data;
                    //
                    #parse_content
                    //
                    // let _ = i; // XXX check if empty?
                    Ok((rem,Self{#(#field_names),*}))
                }
            }
        }
    }
}

fn derive_ber_sequence_content(field_names: &[Ident], asn1_type: Asn1Type) -> TokenStream {
    let from = match asn1_type {
        Asn1Type::Ber => quote! {FromBer::from_ber},
        Asn1Type::Der => quote! {FromDer::from_der},
    };
    let field_parsers: Vec<_> = field_names
        .iter()
        .map(|name| {
            quote! {
                let (i, #name) = #from(i)?;
            }
        })
        .collect();

    quote! {
        #(#field_parsers)*
    }
}
