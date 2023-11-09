//! [![github]](https://github.com/dtolnay/tt-call)&ensp;[![crates-io]](https://crates.io/crates/tt-call)&ensp;[![docs-rs]](https://docs.rs/tt-call)
//!
//! [github]: https://img.shields.io/badge/github-8da0cb?style=for-the-badge&labelColor=555555&logo=github
//! [crates-io]: https://img.shields.io/badge/crates.io-fc8d62?style=for-the-badge&labelColor=555555&logo=rust
//! [docs-rs]: https://img.shields.io/badge/docs.rs-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs
//!
//! <br>
//!
//! **This library is an attempt at seeding an ecosystem of high-quality modular
//! interoperable tt-muncher building blocks.**
//!
//! Tt-munching is a powerful technique for parsing macro\_rules input grammars
//! of significant complexity. In building more and more sophisticated
//! tt-muncher macros it becomes valuable to share code for dealing with certain
//! common input patterns, rather than reimplementing support for those patterns
//! in a low quality and poorly tested way each time.
//!
//! The core macros provided by this library are **[`tt_call!`]** and
//! **[`tt_return!`]**. Together these provide a flexible way to propagate input
//! and output tokens along a recursive descent call hierarchy. One may also
//! view them as a flexible library-only stable implementation of eager
//! expansion for macro\_rules.
//!
//! [`tt_call!`]: macro.tt_call.html
//! [`tt_return!`]: macro.tt_return.html
//!
//! ```toml
//! [dependencies]
//! tt-call = "1.0"
//! ```
//!
//! *Version requirement: tt-call requires a Rust compiler version 1.31 or
//! newer.*
//!
//!
//! ## Calling convention rules
//!
//! - **Macros that conform to tt-call must be invoked with curly braces.**
//!
//!     ```
//!     # macro_rules! some_macro {
//!     #     () => {};
//!     # }
//!     #
//!     some_macro! {
//!         /* ... */
//!     }
//!     ```
//!
//!     The Rust grammar is very particular about punctuation after
//!     parenthesized and square bracketed macro invocations. In expression or
//!     type position they must not be followed by a semicolon. In item or
//!     statement position they are required to be followed by a semicolon. The
//!     inconsistency is applied transitively to any helper macros they forward
//!     to, and means that parenthesized and square bracketed macro invocations
//!     must decide whether to support expression and type position only or item
//!     and statement position only. They cannot support both, which is a
//!     problem for broadly applicable macro building blocks.
//!
//!     There is no such punctuation requirement after curly brace invocations.
//!     Consistently using curly braces makes the same macro building blocks
//!     usable in any syntactic position.
//!
//! - **Input and output values must be passed in the following key-value
//!   form.**
//!
//!     ```
//!     # macro_rules! some_macro {
//!     #     {
//!     $key:ident = [{ $($value:tt)* }]
//!     #     } => {};
//!     # }
//!     ```
//!
//!     This is enforced by the `tt_call!` and `tt_return!` macros. The
//!     consistency is important for composability and makes it possible to
//!     write higher-order macros that operate on the input or output of an
//!     arbitrary tt-call macro.
//!
//!     Except in libraries intended specifically as tt-call building blocks,
//!     generally tt-call macros will be private `#[doc(hidden)]` helpers with a
//!     user-facing non-tt-call entry point. Thus the rigid key-value syntax
//!     need not be exposed to users of the public macro.
//!
//! - **Before its key-value inputs, every rule must accept a `$caller:tt`.**
//!
//!     This is an opaque tt bundle used by `tt_call!` and `tt_return!` to
//!     record the call hierarchy. A `tt_return!` accepts a `$caller` to return
//!     back to.
//!
//! - **Every rule must expand to exactly one macro invocation and nothing
//!   else.**
//!
//!     Output tokens are expected to be returned through `tt_return!`.
//!     Expanding to nothing, expanding to more than one macro invocation, or
//!     expanding to anything other than a macro invocation are not permitted.
//!
//!
//! ## Examples
//!
//! Just as a flavor of the syntax, one of the rules from the implementation of
//! the built-in [`tt_replace!`] macro is written as follows. The macro takes in
//! a token stream and for each token that matches a given predicate it replaces
//! that token with a given replacement sequence of tokens. For example the
//! caller may want to replace the token `self` with the single token `__value`.
//!
//! The rule shown here is responsible for performing one step of the
//! replacement. It matches one token of input in `$first:tt`, uses [`tt_if!`]
//! to invoke the predicate with `$first` as input, recurses with an accumulated
//! copy of the replacement tokens if the predicate returns true, and recurses
//! on the remaining tokens with `$first` preserved unchanged if the predicate
//! returns false.
//!
//! [`tt_replace!`]: macro.tt_replace.html
//! [`tt_if!`]: macro.tt_if.html
//!
//! ```
//! # macro_rules! ignore {
//! {
//!     $caller:tt
//!     condition = [{ $condition:ident }]
//!     replace_with = [{ $($with:tt)* }]
//!     tokens = [{ $($tokens:tt)* }]
//!     rest = [{ $first:tt $($rest:tt)* }]
//! } => {
//!     tt_if! {
//!         condition = [{ $condition }]
//!         input = [{ $first }]
//!         true = [{
//!             private_replace! {
//!                 $caller
//!                 condition = [{ $condition }]
//!                 replace_with = [{ $($with)* }]
//!                 tokens = [{ $($tokens)* $($with)* }]
//!                 rest = [{ $($rest)* }]
//!             }
//!         }]
//!         false = [{
//!             private_replace! {
//!                 $caller
//!                 condition = [{ $condition }]
//!                 replace_with = [{ $($with)* }]
//!                 tokens = [{ $($tokens)* $first }]
//!                 rest = [{ $($rest)* }]
//!             }
//!         }]
//!     }
//! };
//! # }
//! ```
//!
//! Here is another macro rule selected from `tt_replace!`. This one matches if
//! the tt-muncher has reached the end of its input. It returns the finished
//! tokens back to the caller using `tt_return!`.
//!
//! ```
//! # macro_rules! ignore {
//! {
//!     $caller:tt
//!     condition = [{ $condition:ident }]
//!     replace_with = [{ $($with:tt)* }]
//!     tokens = [{ $($tokens:tt)* }]
//!     rest = [{ }]
//! } => {
//!     tt_return! {
//!         $caller
//!         tokens = [{ $($tokens)* }]
//!     }
//! };
//! # }
//! ```
//!
//! One example of a caller-provided predicate for `tt_replace!` could be
//! written as follows. This predicate determines whether the input token is
//! lowercase `self`.
//!
//! ```
//! macro_rules! is_lowercase_self {
//!     // Input token is `self`.
//!     {
//!         $caller:tt
//!         input = [{ self }]
//!     } => {
//!         tt_return! {
//!             $caller
//!             is = [{ true }]
//!         }
//!     };
//!
//!     // Input token is anything other than `self`.
//!     {
//!         $caller:tt
//!         input = [{ $other:tt }]
//!     } => {
//!         tt_return! {
//!             $caller
//!             is = [{ false }]
//!         }
//!     };
//! }
//! ```
//!
//! From here, calling `tt_replace!` with our `is_lowercase_self!` as the
//! condition predicate can be used to implement a fanciful syntax for unary
//! closures: `closure!(self + 1)` should expand to `|__value| __value + 1`.
//!
//! Notice that this user-facing `closure!` macro does not follow the tt-call
//! calling convention. Internally though it uses several tt-call helpers as
//! building blocks.
//!
//! ```
//! # macro_rules! tt_call {
//! #     ($($ignore:tt)*) => {
//! #         2
//! #     };
//! # }
//! #
//! macro_rules! closure {
//!     ($($expr:tt)+) => {
//!         |__value| tt_call! {
//!             macro = [{ tt_replace }]
//!             condition = [{ is_lowercase_self }]
//!             replace_with = [{ __value }]
//!             input = [{ $($expr)+ }]
//!         }
//!     };
//! }
//!
//! fn main() {
//!     let add_one = closure!(self + 1);
//!     println!("{}", add_one(1));
//! }
//! ```
//!
//!
//! ## Motivation
//!
//! This may seem like a lot of ceremony around what should be very simple macro
//! calls. After all, couldn't we write `is_lowercase_self` in a much more
//! straightforward way as follows?
//!
//! ```
//! macro_rules! is_lowercase_self {
//!     (self) => { true };
//!     ($other:tt) => { false };
//! }
//!
//! fn main() {
//!     println!("{}", is_lowercase_self!(self)); // true
//!     println!("{}", is_lowercase_self!(not_self)); // false
//! }
//! ```
//!
//! Qualified yes. As written, the simpler `is_lowercase_self!` behaves as it
//! looks like it should.
//!
//! But suppose we want to build `tt_replace!` or similar macro that needs to
//! invoke `is_lowercase_self!` as a helper. There is no way to do it with this
//! simpler one. No matter what our macro does, there is no way for it to expand
//! `is_lowercase_self!` before expanding itself. If it expands itself first,
//! there is no way for it to use the expansion of `is_lowercase_self!` to
//! decide whether the current token is supposed to be replaced.
//!
//! The `tt_call!` and `tt_return!` abstraction along with `$caller:tt` tracking
//! of the call hierarchy are critical to building composable macros that freely
//! pass around arbitrary tokens and return in a way that can inform expansion
//! of their caller.
//!
//! A future eager expansion feature for declarative macros may render the
//! tt-call approach unnecessary. Eager expansion is listed as an unresolved
//! question in the [tracking issue for declarative macros 2.0][tracking] but is
//! believed to be quite a ways out, if it ever happens. And even then it is not
//! clear whether it is desirable to allow macros expanding to arbitrary tokens.
//! Today macros always expand to an expression, item, statement, type, or
//! pattern. Eager expansion does not automatically mean that the restriction
//! would be lifted to allow a macro that expands to arbitrary tokens such as `!
//! @ #`. The token tree calling convention provides working eager expansion
//! today with support for passing and returning arbitrary token streams.
//!
//! [tracking]: https://github.com/rust-lang/rust/issues/39412
//!
//! And function-like procedural macros once those are stable? It is going to
//! depend on your choice of syntax for the macro input whether a procedural
//! macro is a better choice, but note that they present their own DIY parsing
//! adventures and can be even nastier than tt-call once you get the hang of
//! both. In addition, procedural macros must be defined in a separate crate
//! from the rest of your library so they are not well suited for quick one-off
//! helper macros.
//!
//!
//! ## Design philosphy
//!
//! As may be no surprise by this point, the calling convention design
//! prioritizes scalability and composability over conciseness. A reader
//! familiar with the calling convention (maybe you, six months after writing
//! the macro) should be able to look at any individual tt-call rule by itself
//! and comfortably read off what it does top to bottom and identify its
//! purpose.
//!
//!
//! ## Links
//!
//! - The code that implements `closure!(self + 1)`, all of which is shown
//!   above, can be found all together in [`examples/replace.rs`].
//!
//! - As a more elaborate example of a tt-call macro,
//!   [`examples/comma_separated.rs`] demonstrates a macro that does primitive
//!   name mangling of Rust types. It uses [`parse_type!`] which is a tt-call
//!   version of `$:ty`.
//!
//!     ```
//!     # macro_rules! mangle_type_names {
//!     #     ($($ignore:tt)*) => {
//!     #         &[
//!     #             "_std_fs_File",
//!     #             "_ref_mut_str",
//!     #             "_impl_Display",
//!     #             "_fn_s_ref_str_to_String",
//!     #         ]
//!     #     };
//!     # }
//!     #
//!     static MANGLED: &[&str] = mangle_type_names! {
//!         std::fs::File,
//!         &'a mut str,
//!         impl Display,
//!         fn(s: &str) -> String,
//!     };
//!
//!     fn main() {
//!         assert_eq!(MANGLED, [
//!             "_std_fs_File",
//!             "_ref_mut_str",
//!             "_impl_Display",
//!             "_fn_s_ref_str_to_String",
//!         ]);
//!     }
//!     ```
//!
//! [`examples/replace.rs`]: https://github.com/dtolnay/tt-call/blob/master/examples/replace.rs
//! [`examples/comma_separated.rs`]: https://github.com/dtolnay/tt-call/blob/master/examples/comma_separated.rs
//! [`parse_type!`]: macro.parse_type.html

#![no_std]
#![doc(html_root_url = "https://docs.rs/tt-call/1.0.9")]
#![allow(clippy::module_name_repetitions, clippy::needless_doctest_main)]

mod predicate;
mod replace;
mod rust;
mod unexpected;

// In general it is not possible today in Rust to produce good error messages
// and good error spans at the same time. See:
//
//     https://github.com/rust-lang/rust/issues/44535
//
// Within this crate we prefer to produce errors with the right span, even if
// the message is not good. This scales much better to large input token
// streams.

/// Evaluate a tt-call macro and return its output to a given return
/// destination.
///
/// # Input
///
/// The input must start with an argument called `macro` which provides the name
/// of the macro for `tt_call!` to invoke.
///
///   - `macro = [{` name of macro to call `}]`
///
/// After that there may be any number of key-value pairs to be passed as
/// arguments to the macro being called.
///
///   - **`$(`**<br>
///     &emsp;&emsp;arbitrary key `= [{` arbitrary tokens `}]`<br>
///     **`)*`**
///
/// Finally a specification of the macro invocation to which this call should
/// return its output.
///
///   - `~~>` name of return destination macro `! {`<br>
///     &emsp;&emsp;arbitrary tokens<br>
///     `}`
///
/// # Examples
///
/// ```
/// use tt_call::{tt_call, tt_is_ident};
///
/// macro_rules! print_is_ident {
///     {
///         token = [{ $token:tt }]
///         is_ident = [{ true }]
///     } => {
///         println!("turns out `{}` is an ident", stringify!($token));
///     };
///
///     {
///         token = [{ $token:tt }]
///         is_ident = [{ false }]
///     } => {
///         println!("nope, `{}` is not an ident", stringify!($token));
///     };
/// }
///
/// fn main() {
///     tt_call! {
///         macro = [{ tt_is_ident }]
///         input = [{ foo }]
///         ~~> print_is_ident! {
///             token = [{ foo }]
///         }
///     }
/// }
/// ```
///
/// If the invoked macro provides the entirety of the input to the return
/// destination macro, then the `!` and argument list may be omitted.
///
/// ```
/// use tt_call::{tt_call, tt_is_ident};
///
/// macro_rules! print_is_ident {
///     {
///         is_ident = [{ true }]
///     } => {
///         println!("that token is an ident");
///     };
///
///     {
///         is_ident = [{ false }]
///     } => {
///         println!("nope, not an ident");
///     };
/// }
///
/// fn main() {
///     tt_call! {
///         macro = [{ tt_is_ident }]
///         input = [{ foo }]
///         ~~> print_is_ident
///     }
/// }
/// ```
///
/// And if the invoked macro produces exactly one output value and we just want
/// to expand to that output value, the destination macro may be omitted
/// entirely.
///
/// ```
/// use tt_call::{tt_call, tt_is_ident};
///
/// fn main() {
///     let is_ident = tt_call! {
///         macro = [{ tt_is_ident }]
///         input = [{ foo }]
///     };
///     println!("{}", is_ident); // prints true or false
/// }
/// ```
#[macro_export]
macro_rules! tt_call {
    // Call macro and expand into the tokens of its one return value.
    {
        macro = [{ $($m:ident)::* }]
        $(
            $input:ident = [{ $($tokens:tt)* }]
        )*
    } => {
        $($m)::* ! {
            (__tt_call_private $crate::tt_identity_return! {})
            $(
                $input = [{ $($tokens)* }]
            )*
        }
    };

    // Call macro and pass its return values to the given return destination.
    {
        macro = [{ $($m:ident)::* }]
        $(
            $input:ident = [{ $($tokens:tt)* }]
        )*
        ~~> $($return:ident)::*
    } => {
        $($m)::* ! {
            (__tt_call_private $($return)::* ! {})
            $(
                $input = [{ $($tokens)* }]
            )*
        }
    };

    // Call macro and append its return values onto the invocation of the given
    // return destination without caller.
    {
        macro = [{ $($m:ident)::* }]
        $(
            $input:ident = [{ $($tokens:tt)* }]
        )*
        ~~> $($return:ident)::* ! {
            $(
                $name:ident = [{ $($state:tt)* }]
            )*
        }
    } => {
        $($m)::* ! {
            (__tt_call_private $($return)::* ! {
                $(
                    $name = [{ $($state)* }]
                )*
            })
            $(
                $input = [{ $($tokens)* }]
            )*
        }
    };

    // Call macro and append its return values onto the invocation of the given
    // return destination with caller.
    {
        macro = [{ $($m:ident)::* }]
        $(
            $input:ident = [{ $($tokens:tt)* }]
        )*
        ~~> $($return:ident)::* ! {
            $caller:tt
            $(
                $name:ident = [{ $($state:tt)* }]
            )*
        }
    } => {
        $($m)::* ! {
            (__tt_call_private $($return)::* ! {
                $caller
                $(
                    $name = [{ $($state)* }]
                )*
            })
            $(
                $input = [{ $($tokens)* }]
            )*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! tt_identity_return {
    // Macro returned one value.
    {
        $name:ident = [{ $($output:tt)* }]
    } => {
        $($output)*
    };

    // Macro parsed the entire input and returned one value.
    {
        $name:ident = [{ $($output:tt)* }]
        rest = [{ }]
    } => {
        $($output)*
    };

    // Unexpected: macro failed to parse the entire input.
    {
        $name:ident = [{ $($output:tt)* }]
        rest = [{ $($unexpected:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };
}

/// Return zero or more output values to the caller macro.
///
/// # Input
///
/// The `tt_return!` invocation should be given a `$caller` to return to and a
/// sequence of zero or more named return values.
///
///   - **`$(`**<br>
///     &emsp;&emsp;arbitrary key `= [{` arbitrary tokens `}]`<br>
///     **`)*`**
///
/// # Example
///
/// ```
/// use tt_call::{tt_call, tt_return};
///
/// macro_rules! is_lowercase_self {
///     // Input token is `self`.
///     {
///         $caller:tt
///         input = [{ self }]
///     } => {
///         tt_return! {
///             $caller
///             is = [{ true }]
///         }
///     };
///
///     // Input token is anything other than `self`.
///     {
///         $caller:tt
///         input = [{ $other:tt }]
///     } => {
///         tt_return! {
///             $caller
///             is = [{ false }]
///         }
///     };
/// }
///
/// fn main() {
///     let is = tt_call! {
///         macro = [{ is_lowercase_self }]
///         input = [{ self }]
///     };
///     println!("{}", is);
/// }
/// ```
#[macro_export]
macro_rules! tt_return {
    {
        $caller:tt
        $(
            $output:ident = [{ $($tokens:tt)* }]
        )*
    } => {
        $crate::private_return! {
            $caller
            $(
                $output = [{ $($tokens)* }]
            )*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_return {
    {
        (__tt_call_private $($caller:ident)::* ! { $($state:tt)* })
        $($append:tt)*
    } => {
        $($caller)::* ! {
            $($state)*
            $($append)*
        }
    };
}

/// Evaluate a condition and expand to one or the other of two branches.
///
/// # Input
///
///   - `condition = [{` name of predicate macro to invoke `}]`
///   - `input = [{` arbitrary tokens to pass as input to the predicate `}]`
///   - `true = [{` tokens to expand to if the predicate returns true `}]`
///   - `false = [{` and if the predicate returns false `}]`
///
/// The predicate macro must accept a single input value named `input`. It is
/// expected to return a single output value which may have any name but must
/// hold the tokens `true` or `false`. For example the built-in `tt_is_comma!`
/// predicate expands to `is_comma = [{ true }]` or `is_comma = [{ false }]`.
///
/// # Example
///
/// ```
/// use tt_call::{tt_call, tt_if, tt_is_comma, tt_return};
///
/// macro_rules! parse_until_comma {
///     ($($input:tt)*) => {
///         tt_call! {
///             macro = [{ parse_until_comma_helper }]
///             before_comma = [{ }]
///             tokens = [{ $($input)* }]
///         }
///     };
/// }
///
/// macro_rules! parse_until_comma_helper {
///     {
///         $caller:tt
///         before_comma = [{ $($before:tt)* }]
///         tokens = [{ $first:tt $($rest:tt)* }]
///     } => {
///         tt_if! {
///             condition = [{ tt_is_comma }]
///             input = [{ $first }]
///             true = [{
///                 tt_return! {
///                     $caller
///                     before_comma = [{ $($before)* }]
///                 }
///             }]
///             false = [{
///                 parse_until_comma_helper! {
///                     $caller
///                     before_comma = [{ $($before)* $first }]
///                     tokens = [{ $($rest)* }]
///                 }
///             }]
///         }
///     };
/// }
///
/// fn main() {
///     assert_eq!(3, parse_until_comma!(1 + 2, three, four));
/// }
/// ```
#[macro_export]
macro_rules! tt_if {
    {
        condition = [{ $($condition:ident)::* }]
        input = [{ $($input:tt)* }]
        true = [{ $($then:tt)* }]
        false = [{ $($else:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $($condition)::* }]
            input = [{ $($input)* }]
            ~~> $crate::private_if_branch! {
                true = [{ $($then)* }]
                false = [{ $($else)* }]
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_if_branch {
    // Branch condition returned true.
    {
        true = [{ $($then:tt)* }]
        false = [{ $($else:tt)* }]
        $condition:ident = [{ true }]
    } => {
        $($then)*
    };

    // Branch condition returned false.
    {
        true = [{ $($then:tt)* }]
        false = [{ $($else:tt)* }]
        $condition:ident = [{ false }]
    } => {
        $($else)*
    };
}

/// Print arbitrary output values returned by a tt-call macro. This is valuable
/// for debugging.
/// <sup>**[tt-call]**</sup>
///
/// # Example
///
/// ```
/// use tt_call::{parse_type, tt_call, tt_debug};
///
/// fn main() {
///     tt_call! {
///         macro = [{ parse_type }]
///         input = [{ Vec<u8>, compressed=false }]
///         ~~> tt_debug
///     }
/// }
/// ```
///
/// The output is:
///
/// ```text
/// type = [{ Vec < u8 > }]
/// rest = [{ , compressed = false }]
/// ```
#[macro_export]
macro_rules! tt_debug {
    {
        $(
            $output:ident = [{ $($tokens:tt)* }]
        )*
    } => {
        $(
            println!(
                "{}",
                concat!(
                    stringify!($output),
                    " = [{ ",
                    stringify!($($tokens)*),
                    " }]",
                )
            );
        )*
    }
}
