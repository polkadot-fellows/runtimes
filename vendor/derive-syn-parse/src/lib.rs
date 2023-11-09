//! Derive macro for [`syn::parse::Parse`]
//!
//! A common pattern when writing custom `syn` parsers is repeating `<name>: input.parse()?` for
//! each field in the output. `#[derive(Parse)]` handles that for you, with some extra helpful
//! customization.
//!
//! ## Usage
//!
//! Using this crate is as simple as adding it to your 'Cargo.toml' and importing the derive macro:
//!
//! ```toml
//! # Cargo.toml
//!
//! [dependencies]
//! derive-syn-parse = "0.1.5"
//! ```
//!
//! ```
//! // your_file.rs
//! use derive_syn_parse::Parse;
//!
//! #[derive(Parse)]
//! struct CustomParsable {
//!     // ...
//! }
//! ```
//!
//! The derived implementation of `Parse` always parses in the order that the fields are given.
//! **Note that deriving `Parse` is also available on enums.** For more information, see the
//! [dedicated section](#enum-parsing).
//!
//! This crate is intended for users who are already making heavy use of `syn`.
//!
//! ## Motivation
//!
//! When writing rust code that makes heavy use of `syn`'s parsing functionality, we often end up
//! writing things like:
//! ```
//! use syn::parse::{Parse, ParseStream};
//! use syn::{Ident, Token, Type};
//!
//! // A simplified struct field
//! //
//! //     x: i32
//! struct MyField {
//!     ident: Ident,
//!     colon_token: Token![:],
//!     ty: Type,
//! }
//!
//! impl Parse for MyField {
//!     fn parse(input: ParseStream) -> syn::Result<Self> {
//!         Ok(MyField {
//!             ident: input.parse()?,
//!             colon_token: input.parse()?,
//!             ty: input.parse()?,
//!         })
//!     }
//! }
//! ```
//! This is really repetitive! Ideally, we'd like to just `#[derive(Parse)]` and have it work. And
//! so we can! (for the most part) Adding `#[derive(Parse)]` to the previous struct produces an
//! equivalent implementation of `Parse`:
//! ```
//! use syn::{Ident, Token, Type};
//! use derive_syn_parse::Parse;
//!
//! #[derive(Parse)]
//! struct MyField {
//!     ident: Ident,
//!     colon_token: Token![:],
//!     ty: Type,
//! }
//! ```
//!
//! Of course, there are more complicated cases. This is mainly covered below in the 'Advanced
//! Usage' section.
//!
//! ## Advanced Usage
//!
//! There are a few different facilities provided here, including:
//! * Enum variant parsing,
//! * Conditional field parsing,
//! * Parsing within token trees (parens, brackets, etc.),
//! * And much more!
//!
//! Each of the below sections can be expanded to view detailed information about how to use a
//! particular component. Be warned - each section assumes a fair amount of knowledge about the relevant
//! `syn` features.
//
//
// ---------- SECTION: Enum Parsing ----------
//
//! <details><summary><b>➤ Enum parsing</b></summary>
//!
//! Parsing enums is a complex feature. When writing manual implementations of
//! `Parse`, it doesn't come up as often, but there are also typically *many* ways to do it:
//! `syn` provides both forking the `ParseBuffer` *and* peeking to handle this, with the suggestion that
//! peeking be preferred.
//!
//! This library does not support forking; it tends to suffer from poor error messages and general
//! inefficiency. That being said, manual implementations of `Parse` can and should be written in the
//! rare cases when this library is insufficient.
//!
//! We support peeking in a couple differnet ways - with the `#[peek]` and `#[peek_with]` attributes.
//! One peeking attribute is required for each `enum` variant. The general syntax tends to look like:
//!
//! ```text
//! #[peek($TYPE, name = $NAME)]
//! ```
//! and
//! ```text
//! #[peek_with($EXPR, name = $NAME)]
//! ```
//! The name is provided in order to construct useful error messages when input doesn't match any of the
//! variants.
//!
//! These essentially translate to:
//! ```
//! if input.peek($TYPE) {
//!     // parse variant
//! } else {
//!     // parse other variants
//! }
//! ```
//! and
//! ```
//! if ($EXPR)(input) {
//!     // parse variant
//! } else {
//!     // parse other variants
//! }
//! ```
//! </details>
//
//
// ---------- SECTION: Token Trees ----------
//
//! <details><summary><b>➤ Token Trees (parens, brackets, braces)</b></summary>
//!
//! If derive macros had access to type information, we could auto-detect when a field contains any
//! of `syn::token::{Paren, Bracket, Brace}`. Unfortunately, we can't - and these don't implement
//! `Parse`, so they each have their own special attribute to mark them: `#[paren]`, `#[bracket]`,
//! and `#[brace]`, respectively.
//!
//! These are typically used with the `#[inside]` attribute, which indicates that a field should be
//! parsed inside a particular named token tree. This might look like:
//!
//! ```
//! use derive_syn_parse::Parse;
//! use syn::{Ident, token, Expr};
//!
//! // Parses a simple function call - something like
//! //
//! //   so_long(and_thanks + for_all * the_fish)
//! #[derive(Parse)]
//! struct SingleArgFn {
//!     name: Ident,
//!     #[paren]
//!     arg_paren: token::Paren,
//!     #[inside(arg_paren)]
//!     arg: Expr,
//! }
//! ```
//!
//! The `#[inside]` attribute can - of course - be repeated with multiple token trees, though this
//! may not necessarily produce the most readable type definitions.
//!
//! For reference, the above code produces an implementation equivalent to:
//! ```
//! # use syn::{Ident, token, Expr};
//! # struct SingleArgFn { name: Ident, arg_paren: token::Paren, arg: Expr }
//!
//! use syn::parse::{Parse, ParseStream};
//!
//! impl Parse for SingleArgFn {
//!     fn parse(input: ParseStream) -> syn::Result<Self> {
//!         let paren;
//!         Ok(SingleArgFn {
//!             name: input.parse()?,
//!             arg_paren: syn::parenthesized!(paren in input),
//!             arg: paren.parse()?,
//!         })
//!     }
//! }
//! ```
//!
//! </details>
//
//
// ---------- SECTION: Custom parsing (call, parse_terminated) ----------
//
//! <details><summary><b>➤ Custom parse functions (<code>#[call]</code>, <code>#[parse_terminated]</code>)</b></summary>
//!
//! Not every type worth parsing implements `Parse`, but we still might want to parse them - things
//! like [`Vec<Attribute>`] or any [`Punctuated<_, _>`] type. In these cases, the available
//! attributes mirror the methods on [`ParseBuffer`].
//!
//! [`ParseBuffer`]: syn::parse::ParseBuffer
//!
//! For `#[parse_terminated]`, there aren't any parameters that can be specified - it's common
//! enough that it's provided for those `Punctuated` fields.
//!
//! Alternatively, `#[call]` has the syntax `#[call( EXPR )]`, where `EXPR` is *any expression*
//! implementing `FnOnce(ParseBuffer) -> syn::Result<T>`. Typically, this might be something like:
//! ```
//! use syn::{Attribute, Ident, Token};
//!
//! // Parses a unit struct with attributes.
//! //
//! //     #[derive(Copy, Clone)]
//! //     struct S;
//! #[derive(Parse)]
//! struct UnitStruct {
//!     #[call(Attribute::parse_outer)]
//!     attrs: Vec<Attribute>,
//!     struct_token: Token![struct],
//!     name: Ident,
//!     semi_token: Token![;],
//! }
//! ```
//!
//! Unlike with [`ParseBuffer::call`], which only accepts functions that are
//! `fn(ParseStream) -> syn::Result<T>`, `#[call]` allows any expression that we can call with the
//! `ParseBuffer`. So one could - hypothetically - implement `#[parse_if]` with this:
//! ```
//! struct Foo {
//!     a: Option<Token![=>]>,
//!     #[call(|inp| match &a { Some(_) => Ok(Some(inp.parse()?)), None => Ok(None) })]
//!     b: Option<Bar>,
//! }
//! ```
//! Though it's probably best to just use `#[parse_if]` :)
//!
//! [`Vec<Attribute>`]: syn::Attribute
//! [`Punctuated<_, _>`]: syn::punctuated::Punctuated
//! [`ParseBuffer::call`]: syn::parse::ParseBuffer
//!
//! </details>
//
//
// ---------- SECTION: Conditional field parsing ----------
//
//! <details><summary><b>➤ Conditional field parsing (<code>#[parse_if]</code>, <code>#[peek]</code>)</b></summary>
//!
//! When implementing `Parse` for structs, it is occasionally the case that certain fields are
//! optional - or should only be parsed under certain circumstances. There are attributes for that!
//!
//! Say we want to parse enums with the following, different syntax:
//!
//! ```
//! enum Foo {
//!     Bar: Baz,
//!     Qux,
//! }
//! ```
//! where the equivalent Rust code would be:
//!
//! ```
//! enum Foo {
//!     Bar(Baz),
//!     Qux,
//! }
//! ```
//! There’s two ways we could parse the variants here – either with a colon and following type or
//! with no colon or type. To handle this, we can write:
//!
//! ```
//! #[derive(Parse)]
//! struct Variant {
//!     name: Ident,
//!     // `syn` already supports optional parsing of simple tokens
//!     colon: Option<Token![:]>,
//!     // We only want to parse the trailing type if there's a colon:
//!     #[parse_if(colon.is_some())]
//!     ty: Option<Type>,
//! }
//! ```
//! Note that in this case, `ty` must be an `Option`. In addition to conditional parsing based on
//! the values of what’s already been parsed, we can also peek - just as described above in the
//! section on parsing enums. The only difference here is that we do not need to provide a name for
//! the optional field. We could have equally implemented the above as:
//!
//! ```
//! #[derive(Parse)]
//! struct Variant {
//!     name: Ident,
//!     #[peek(Token![:])]
//!     ty: Option<VariantType>,
//! }
//!
//! #[derive(Parse)]
//! struct VariantType {
//!     colon: Token![:],
//!     ty: Type,
//! }
//! ```
//!
//! </details>
//
//
// ---------- SECTION: Prefix & postfix ----------
//
//! <details> <summary><b>➤ Temporary parses: Prefix & postfix </b></summary>
//!
//! A common pattern that sometimes occurs when deriving `Parse` implementations is to have many
//! unused punctuation fields - imagine a hypothetical implementation of field parsing with default
//! values:
//!
//! ```
//! // A field with default values, parsing something like:
//! //
//! //   foo: Bar = Bar::new()
//! #[derive(Parse)]
//! struct Field {
//!     ident: Ident,
//!     colon: Token![:],
//!     ty: Type,
//!     eq: Option<Token![=]>,
//!     #[parse_if(eq.is_some())]
//!     expr: Option<Expr>,
//! }
//! ```
//!
//! Here, there's a couple fields that probably won't be used later - both `colon` and `eq`. We can
//! elimitate both of these with the `#[prefix]` attribute:
//!
//! ```
//! // A field with default values, parsing something like:
//! //
//! //   foo: Bar = Bar::new()
//! #[derive(Parse)]
//! struct Field {
//!     ident: Ident,
//!     #[prefix(Token![:])]
//!     ty: Type,
//!     #[prefix(Option<Token![=]> as eq)]
//!     #[parse_if(eq.is_some())]
//!     expr: Option<Expr>,
//! }
//! ```
//!
//! We can use `"as <Ident>"` to give a temporary name to the value - including it as a parsed
//! value that can be referenced in other parsing clauses, but without adding it as a struct field.
//!
//! There's *also* a `#[postfix]` attribute, which operates very similarly to `#[prefix]`, but
//! exists to allow unused fields at the end of the struct. In general, `#[postfix]` tends to be
//! pretty tricky to read, so it's generally preferable to use `#[prefix]` to keep the field
//! ordering the same as the parse order.
//!
//! In some cases, we might want to have both a field and its prefix parsed inside some other token
//! tree. Like the following contrived example:
//!
//! ```
//! use syn::*;
//!
//! // Parses.... something. Who knows if this is useful... :P
//! //
//! //   (=> x + 2)
//! #[derive(Parse)]
//! struct Funky {
//!     #[paren]
//!     paren: token::Paren,
//!     #[inside(paren)]
//!     r_arrow: Token![=>],
//!     #[inside(paren)]
//!     expr: Expr,
//! }
//! ```
//!
//! To remove the unused `r_arrow` field here, we have an other extra piece we can add:
//! `"in" <Ident>"`.
//!
//! ```
//! #[derive(Parse)]
//! struct Funky {
//!     #[paren]
//!     paren: token::Paren,
//!     #[prefix(Token![=>] in paren)]
//!     #[inside(paren)]
//!     expr: Expr,
//! }
//! ```
//!
//! Note that attempting to write the `#[inside]` before `#[prefix]` is forbidden; it's less clear
//! what the expected behavior there should be.
//!
//! Finally, when combining both `"as" <ident>` and `"in" <ident>`, they should come in that
//! order - e.g. `#[prefix(Foo as bar in baz)]`.
//!
//! </details>

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Ident, Result};

#[macro_use]
mod error_macros;

mod fields;
#[cfg(test)]
mod tests;
mod variants;

#[rustfmt::skip]
#[proc_macro_derive(
    Parse,
    attributes(
        paren, bracket, brace,
        inside,
        call, parse_terminated,
        peek, peek_with,
        parse_if,
        prefix, postfix,
    )
)]
pub fn derive_parse(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    derive_parse_internal(input).into()
}

// Pulled into a separate function so we can test it
pub(crate) fn derive_parse_internal(input: DeriveInput) -> TokenStream {
    // The generic parameters following `impl`
    let mut generics_intro = TokenStream::new();
    // The generic arguments following the name of the type
    let mut generics_args = TokenStream::new();

    let where_clause = input.generics.where_clause;

    let generic_params: Vec<_> = input.generics.params.into_iter().collect();
    if !generic_params.is_empty() {
        let generics_intros: Vec<_> = handle_syn_result! {
            generic_params.iter()
                .map(require_impl_parse_if_type)
                .collect()
        };
        generics_intro = quote!( < #( #generics_intros ),* > );
        let generics_args_list: Vec<_> = generic_params.into_iter().map(convert_to_arg).collect();
        generics_args = quote!( < #( #generics_args_list ),* > );
    }

    let ident = input.ident;

    let parse_impl = match input.data {
        Data::Union(u) => invalid_input_kind!(u.union_token),
        Data::Struct(s) => handle_syn_result! {
            @default_impl_from(generics_intro, ident, generics_args, where_clause),
            fields::generate_fn_body(&ident, s.fields, false)
        },
        Data::Enum(e) => handle_syn_result! {
            @default_impl_from(generics_intro, ident, generics_args, where_clause),
            variants::generate_impl(e.variants.into_iter())
        },
    };

    let parse_input = parse_input();
    quote!(
        impl #generics_intro ::syn::parse::Parse for #ident #generics_args #where_clause {
            fn parse(#parse_input: ::syn::parse::ParseStream) -> ::syn::Result<Self> {
                #parse_impl
            }
        }
    )
}

// Produces the tokens for the generic parameter, adding `+ syn::parse::Parse`
fn require_impl_parse_if_type(param: &syn::GenericParam) -> Result<TokenStream> {
    use syn::GenericParam::Type;
    use syn::TypeParam;

    let TypeParam {
        attrs,
        ident,
        colon_token,
        bounds,
        eq_token,
        default,
    } = match param {
        Type(t) => t,
        param => return Ok(param.to_token_stream()),
    };

    // If we have `struct Foo<T>`,      we need to add `: Parse`, but
    // if we have `struct Foo<T: Bar>`, we need to add `+ Parse`
    let parse_bound = if colon_token.is_some() {
        quote_spanned! {
            ident.span()=>
            + ::syn::parse::Parse
        }
    } else {
        quote_spanned! {
            ident.span()=>
            : ::syn::parse::Parse
        }
    };

    Ok(quote! {
        #( #attrs )*
        #ident #colon_token #bounds #parse_bound #eq_token #default
    })
}

fn convert_to_arg(param: syn::GenericParam) -> TokenStream {
    use syn::GenericParam::{Const, Lifetime, Type};

    match param {
        Type(ty) => ty.ident.to_token_stream(),
        Lifetime(lifetime) => lifetime.to_token_stream(),
        Const(con) => {
            let ident = &con.ident;
            quote_spanned!(con.span()=> { #ident })
        }
    }
}

// A helper macro to give the identifier used to represent the ParseStream used as input to the
// macro
fn parse_input() -> Ident {
    Ident::new("__parse_input", Span::call_site())
}
