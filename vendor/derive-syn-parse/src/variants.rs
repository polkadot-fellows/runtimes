//! Handling for generating a `Parse` implementation from `enum` variants

use crate::fields::generate_fn_body;
use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::token;
use syn::{parenthesized, Attribute, Expr, Ident, LitStr, Result, Token, Variant};

pub(crate) fn generate_impl(
    variants: impl ExactSizeIterator<Item = Variant>,
) -> Result<TokenStream> {
    // generate each `peek` and corresponding inner implementation

    if variants.len() == 0 {
        return Err(syn::Error::new(
            Span::call_site(),
            "cannot derive `Parse` for an empty `enum`",
        ));
    }

    let input_source = crate::parse_input();

    let mut names = Vec::new();
    let conditional_return_if_peek_success = variants
        .into_iter()
        .map(|var| {
            let (name, if_condition) = impl_for_variant(&input_source, var)?;
            names.push(name);
            Ok(if_condition)
        })
        .collect::<Result<Vec<_>>>()?;

    let error_msg = implemented_error_msg(names);

    Ok(quote! {
        #( #conditional_return_if_peek_success )*

        Err(#input_source.error(#error_msg))
    })
}

fn implemented_error_msg(names: Vec<LitStr>) -> String {
    let one_of = match names.len() {
        1 => "",
        2 => "either ",
        _ => "one of ",
    };

    let name_list = match names.len() {
        0 => unreachable!(),
        1 => names[0].value(),
        2 => format!("{} or {}", names[0].value(), names[1].value()),
        _ => {
            let middle = names[1..names.len() - 1]
                .iter()
                .map(|name| format!(", {}", name.value()))
                .collect::<String>();
            format!(
                "{}{}, or {}",
                names[0].value(),
                middle,
                names.last().unwrap().value()
            )
        }
    };

    format!("expected {}{}", one_of, name_list)
}

enum VariantAttr {
    Peek(PeekInfo),
    PeekWith(PeekInfo),
}

mod kwd {
    syn::custom_keyword!(name);
}

struct PeekInfo {
    _paren_token: token::Paren,
    expr: Expr,
    _comma: Token![,],
    _name_token: kwd::name,
    _eq: Token![=],
    name: LitStr,
}

// If successful, the first element in the tuple is the name to use to refer to the variant in
// error messages. The second element is an `if` expression that looks something like:
//
//   if $input_source.peek($peek_value) {
//       Ok(Self::$variant_name {
//          $( $field: $input_source.parse()?, )*
//       })
//   }
fn impl_for_variant(input_source: &Ident, variant: Variant) -> Result<(LitStr, TokenStream)> {
    use VariantAttr::{Peek, PeekWith};

    let variant_span = variant.span();

    let diagnositc_name: LitStr;
    let peek_expr = match extract_single_attr(variant_span, variant.attrs)? {
        Peek(PeekInfo { expr, name, .. }) => {
            diagnositc_name = name;
            quote_spanned! {
                expr.span()=>
                #input_source.peek(#expr)
            }
        }
        PeekWith(PeekInfo { expr, name, .. }) => {
            diagnositc_name = name;
            quote_spanned! {
                expr.span()=>
                (#expr)(#input_source)
            }
        }
    };

    let ident = variant.ident;
    let variant_path = quote!( Self::#ident );
    let parse_implementation = generate_fn_body(&variant_path, variant.fields, true)?;

    let output = quote! {
        if #peek_expr {
            #parse_implementation;
        }
    };

    Ok((diagnositc_name, output))
}

fn extract_single_attr(variant_span: Span, attrs: Vec<Attribute>) -> Result<VariantAttr> {
    let mut attrs: Vec<_> = attrs
        .into_iter()
        .filter_map(try_as_variant_attr)
        .collect::<Result<_>>()?;

    match attrs.len() {
        0 => Err(syn::Error::new(
            variant_span,
            "enum variants must have `#[peek(..)]` or `#[peek_with(..)]` to derive `Parse`",
        )),
        1 => Ok(attrs.remove(0)),
        _ => Err(syn::Error::new(
            variant_span,
            "more than one peeking attribute is disallowed; please use `#[peek_with(..)]` for a custom function",
        )),
    }
}

fn try_as_variant_attr(attr: Attribute) -> Option<Result<VariantAttr>> {
    let name = attr.path.get_ident()?.to_string();

    match name.as_str() {
        "peek" => Some(syn::parse2(attr.tokens).map(VariantAttr::Peek)),
        "peek_with" => Some(syn::parse2(attr.tokens).map(VariantAttr::PeekWith)),
        _ => None,
    }
}

////////////////////////////////////////////
// Boilerplate `Parse` implementations 🙃 //
////////////////////////////////////////////

impl Parse for PeekInfo {
    fn parse(input: ParseStream) -> Result<Self> {
        let paren;
        Ok(PeekInfo {
            _paren_token: parenthesized!(paren in input),
            expr: paren.parse()?,
            _comma: paren.parse()?,
            _name_token: paren.parse()?,
            _eq: paren.parse()?,
            name: paren.parse()?,
        })
    }
}
