[![crates.io](https://img.shields.io/crates/v/derive-syn-parse.svg)](https://crates.io/crates/derive-syn-parse)
[![docs.rs](https://docs.rs/derive-syn-parse/badge.svg)](https://docs.rs/derive-syn-parse)

# `derive-syn-parse`: A derive macro for `syn`'s `Parse` trait

This is a fairly straightforward derive macro that produces an implementation `syn::parse::Parse`
for the type it's applied to.

A common pattern when writing custom `syn` parsers is repeating `<name>: input.parse()?` for
each field in the output. This crate's `#[derive(Parse)]` handles that for you, with some helpful
extra customization.

## Usage

Using this crate is as simple as adding it to your 'Cargo.toml' and importing the derive macro:

```toml
# Cargo.toml

[dependencies]
derive-syn-parse = "0.1.5"
```

```rust
// your_file.rs
use derive_syn_parse::Parse;

#[derive(Parse)]
struct CustomParsable {
    // ...
}
```

The derived implementation of `Parse` will always parse in the order that the fields are given.
Detailed information about the various field attributes available is given in the
[crate documentation](https://docs.rs/derive-syn-parse).

This crate is primarily intended for users who are already making heavy use of `syn` and wish to
reduce the amount of boilerplate code required.

## Motivation

When writing rust code that makes heavy use of `syn`'s parsing functionality, we often end up
writing things like:
```rust
use syn::parse::{Parse, ParseStream};
use syn::{Ident, Token, Type};

// A simplified struct field
//
//     x: i32
struct MyField {
    ident: Ident,
    colon_token: Token![:],
    ty: Type,
}

impl Parse for MyField {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(MyField {
            ident: input.parse()?,
            colon_token: input.parse()?,
            ty: input.parse()?,
        })
    }
}
```
This is really repetitive! Ideally, we'd like to just `#[derive(Parse)]` and have it work. And
so we can! (for the most part) Adding `#[derive(Parse)]` to the previous struct produces an
equivalent implementation of `Parse`:
```rust
use syn::{Ident, Token, Type};
use derive_syn_parse::Parse;

#[derive(Parse)]
struct MyField {
    ident: Ident,
    colon_token: Token![:],
    ty: Type,
}
```

Of course, there are more complicated cases. But - even though they're complicated, many of them are
still covered by the various advanced features provided! For more information, see the
[crate documentation](https://docs.rs/derive-syn-parse).
