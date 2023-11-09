//! This example demonstrates a tt-call macro for parsing comma-separated
//! applications of an arbitrary given tt-call element. The way it is invoked
//! here, the elements are Rust types parsed by the built-in `parse_type!`
//! macro.

#![recursion_limit = "256"]

use tt_call::{parse_type, tt_call, tt_return};

/// Parse comma-separated elements.
/// <sup>**[tt-call]**</sup>
///
/// Trailing comma is accepted but optional.
///
/// # Input
///
///   - `parser = [{` the parser macro with which to parse each element `}]`
///   - `input = [{` arbitrary input tokens to parse `}]`
///
/// # Output
///
///   - `element = [{` parsed element (repeated zero or more times) `}]`
///
/// # Example
///
/// ```
/// tt_call! {
///     macro = [{ comma_separated }]
///     parser = [{ parse_type }]
///     input = [{
///         std::fs::File,
///         &'a mut str,
///     }]
///     ~~> tt_debug
/// }
/// ```
///
/// ```console
/// element = [{ std :: fs :: File }]
/// element = [{ & 'a mut str }]
/// ```
macro_rules! comma_separated {
    {
        $caller:tt
        parser = [{ $parser:ident }]
        input = [{ $($input:tt)* }]
    } => {
        tt_call! {
            macro = [{ $parser }]
            input = [{ $($input)* }]
            ~~> private_comma_separated! {
                $caller
                parser = [{ $parser }]
                elements = [{ }]
            }
        }
    };
}

#[doc(hidden)]
macro_rules! private_comma_separated {
    // Finished without trailing comma.
    {
        $caller:tt
        parser = [{ $parser:ident }]
        elements = [{ $($elements:tt)* }]
        $name:ident = [{ $($current:tt)* }]
        rest = [{ }]
    } => {
        tt_return! {
            $caller
            $($elements)*
            element = [{ $($current)* }]
        }
    };

    // Finished after ignoring trailing comma.
    {
        $caller:tt
        parser = [{ $parser:ident }]
        elements = [{ $($elements:tt)* }]
        $name:ident = [{ $($current:tt)* }]
        rest = [{ , }]
    } => {
        tt_return! {
            $caller
            $($elements)*
            element = [{ $($current)* }]
        }
    };

    // Parse next element after comma.
    {
        $caller:tt
        parser = [{ $parser:ident }]
        elements = [{ $($elements:tt)* }]
        $name:ident = [{ $($current:tt)* }]
        rest = [{ , $($rest:tt)+ }]
    } => {
        tt_call! {
            macro = [{ $parser }]
            input = [{ $($rest)* }]
            ~~> private_comma_separated! {
                $caller
                parser = [{ $parser }]
                elements = [{
                    $($elements)*
                    element = [{ $($current)* }]
                }]
            }
        }
    };
}

/// Parses a comma-separated sequence of Rust types and expands to a `&'static
/// [&'static str]` containing a basic mangled name for each input type.
macro_rules! mangle_type_names {
    ($($input:tt)*) => {
        tt_call! {
            macro = [{ comma_separated }]
            parser = [{ parse_type }]
            input = [{ $($input)* }]
            ~~> private_mangle_type_names
        }
    };
}

#[doc(hidden)]
macro_rules! private_mangle_type_names {
    {
        $(
            element = [{ $($element:tt)* }]
        )*
    } => {
        &[
            $(
                concat!(
                    $(
                        mangle_token!($element),
                    )*
                ),
            )*
        ]
    };
}

#[doc(hidden)]
macro_rules! mangle_token {
    ($ident:ident) => {
        concat!("_", stringify!($ident))
    };

    (&) => {
        "_ref"
    };

    (->) => {
        "_to"
    };

    (($($parenthesized:tt)*)) => {
        concat!(
            $(
                mangle_token!($parenthesized),
            )*
        )
    };

    /* more sorts of tokens may be handled here if necessary */

    ($other:tt) => {
        ""
    };
}

fn main() {
    static MANGLED: &[&str] = mangle_type_names! {
        std::fs::File,
        &'a mut str,
        impl Display,
        fn(s: &str) -> String,
    };

    assert_eq!(
        MANGLED,
        [
            "_std_fs_File",
            "_ref_mut_str",
            "_impl_Display",
            "_fn_s_ref_str_to_String",
        ]
    );

    println!("{:#?}", MANGLED);
}
