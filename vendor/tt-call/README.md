Token Tree Calling Convention
=============================

[<img alt="github" src="https://img.shields.io/badge/github-dtolnay/tt--call-8da0cb?style=for-the-badge&labelColor=555555&logo=github" height="20">](https://github.com/dtolnay/tt-call)
[<img alt="crates.io" src="https://img.shields.io/crates/v/tt-call.svg?style=for-the-badge&color=fc8d62&logo=rust" height="20">](https://crates.io/crates/tt-call)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-tt--call-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="20">](https://docs.rs/tt-call)
[<img alt="build status" src="https://img.shields.io/github/actions/workflow/status/dtolnay/tt-call/ci.yml?branch=master&style=for-the-badge" height="20">](https://github.com/dtolnay/tt-call/actions?query=branch%3Amaster)

**This library is an attempt at seeding an ecosystem of high-quality modular
interoperable tt-muncher building blocks.**

Tt-munching is a powerful technique for parsing macro\_rules input grammars
of significant complexity. In building more and more sophisticated
tt-muncher macros it becomes valuable to share code for dealing with certain
common input patterns, rather than reimplementing support for those patterns
in a low quality and poorly tested way each time.

The core macros provided by this library are **[`tt_call!`]** and
**[`tt_return!`]**. Together these provide a flexible way to propagate input
and output tokens along a recursive descent call hierarchy. One may also
view them as a flexible library-only stable implementation of eager
expansion for macro\_rules.

[`tt_call!`]: https://docs.rs/tt-call/1.0/tt_call/macro.tt_call.html
[`tt_return!`]: https://docs.rs/tt-call/1.0/tt_call/macro.tt_return.html

```toml
[dependencies]
tt-call = "1.0"
```

*Version requirement: tt-call requires a Rust compiler version 1.31 or
newer.*


## Calling convention rules

- **Macros that conform to tt-call must be invoked with curly braces.**

    ```rust
    some_macro! {
        /* ... */
    }
    ```

    The Rust grammar is very particular about punctuation after
    parenthesized and square bracketed macro invocations. In expression or
    type position they must not be followed by a semicolon. In item or
    statement position they are required to be followed by a semicolon. The
    inconsistency is applied transitively to any helper macros they forward
    to, and means that parenthesized and square bracketed macro invocations
    must decide whether to support expression and type position only or item
    and statement position only. They cannot support both, which is a
    problem for broadly applicable macro building blocks.

    There is no such punctuation requirement after curly brace invocations.
    Consistently using curly braces makes the same macro building blocks
    usable in any syntactic position.

- **Input and output values must be passed in the following key-value
  form.**

    ```rust
    $key:ident = [{ $($value:tt)* }]
    ```

    This is enforced by the `tt_call!` and `tt_return!` macros. The
    consistency is important for composability and makes it possible to
    write higher-order macros that operate on the input or output of an
    arbitrary tt-call macro.

    Except in libraries intended specifically as tt-call building blocks,
    generally tt-call macros will be private `#[doc(hidden)]` helpers with a
    user-facing non-tt-call entry point. Thus the rigid key-value syntax
    need not be exposed to users of the public macro.

- **Before its key-value inputs, every rule must accept a `$caller:tt`.**

    This is an opaque tt bundle used by `tt_call!` and `tt_return!` to
    record the call hierarchy. A `tt_return!` accepts a `$caller` to return
    back to.

- **Every rule must expand to exactly one macro invocation and nothing
  else.**

    Output tokens are expected to be returned through `tt_return!`.
    Expanding to nothing, expanding to more than one macro invocation, or
    expanding to anything other than a macro invocation are not permitted.


## Examples

Just as a flavor of the syntax, one of the rules from the implementation of
the built-in [`tt_replace!`] macro is written as follows. The macro takes in
a token stream and for each token that matches a given predicate it replaces
that token with a given replacement sequence of tokens. For example the
caller may want to replace the token `self` with the single token `__value`.

The rule shown here is responsible for performing one step of the
replacement. It matches one token of input in `$first:tt`, uses [`tt_if!`]
to invoke the predicate with `$first` as input, recurses with an accumulated
copy of the replacement tokens if the predicate returns true, and recurses
on the remaining tokens with `$first` preserved unchanged if the predicate
returns false.

[`tt_replace!`]: https://docs.rs/tt-call/1.0/tt_call/macro.tt_replace.html
[`tt_if!`]: https://docs.rs/tt-call/1.0/tt_call/macro.tt_if.html

```rust
{
    $caller:tt
    condition = [{ $condition:ident }]
    replace_with = [{ $($with:tt)* }]
    tokens = [{ $($tokens:tt)* }]
    rest = [{ $first:tt $($rest:tt)* }]
} => {
    tt_if! {
        condition = [{ $condition }]
        input = [{ $first }]
        true = [{
            private_replace! {
                $caller
                condition = [{ $condition }]
                replace_with = [{ $($with)* }]
                tokens = [{ $($tokens)* $($with)* }]
                rest = [{ $($rest)* }]
            }
        }]
        false = [{
            private_replace! {
                $caller
                condition = [{ $condition }]
                replace_with = [{ $($with)* }]
                tokens = [{ $($tokens)* $first }]
                rest = [{ $($rest)* }]
            }
        }]
    }
};
```

Here is another macro rule selected from `tt_replace!`. This one matches if
the tt-muncher has reached the end of its input. It returns the finished
tokens back to the caller using `tt_return!`.

```rust
{
    $caller:tt
    condition = [{ $condition:ident }]
    replace_with = [{ $($with:tt)* }]
    tokens = [{ $($tokens:tt)* }]
    rest = [{ }]
} => {
    tt_return! {
        $caller
        tokens = [{ $($tokens)* }]
    }
};
```

One example of a caller-provided predicate for `tt_replace!` could be
written as follows. This predicate determines whether the input token is
lowercase `self`.

```rust
macro_rules! is_lowercase_self {
    // Input token is `self`.
    {
        $caller:tt
        input = [{ self }]
    } => {
        tt_return! {
            $caller
            is = [{ true }]
        }
    };

    // Input token is anything other than `self`.
    {
        $caller:tt
        input = [{ $other:tt }]
    } => {
        tt_return! {
            $caller
            is = [{ false }]
        }
    };
}
```

From here, calling `tt_replace!` with our `is_lowercase_self!` as the
condition predicate can be used to implement a fanciful syntax for unary
closures: `closure!(self + 1)` should expand to `|__value| __value + 1`.

Notice that this user-facing `closure!` macro does not follow the tt-call
calling convention. Internally though it uses several tt-call helpers as
building blocks.

```rust
macro_rules! closure {
    ($($expr:tt)+) => {
        |__value| tt_call! {
            macro = [{ tt_replace }]
            condition = [{ is_lowercase_self }]
            replace_with = [{ __value }]
            input = [{ $($expr)+ }]
        }
    };
}

fn main() {
    let add_one = closure!(self + 1);
    println!("{}", add_one(1));
}
```


## Motivation

This may seem like a lot of ceremony around what should be very simple macro
calls. After all, couldn't we write `is_lowercase_self` in a much more
straightforward way as follows?

```rust
macro_rules! is_lowercase_self {
    (self) => { true };
    ($other:tt) => { false };
}

fn main() {
    println!("{}", is_lowercase_self!(self)); // true
    println!("{}", is_lowercase_self!(not_self)); // false
}
```

Qualified yes. As written, the simpler `is_lowercase_self!` behaves as it
looks like it should.

But suppose we want to build `tt_replace!` or similar macro that needs to
invoke `is_lowercase_self!` as a helper. There is no way to do it with this
simpler one. No matter what our macro does, there is no way for it to expand
`is_lowercase_self!` before expanding itself. If it expands itself first,
there is no way for it to use the expansion of `is_lowercase_self!` to
decide whether the current token is supposed to be replaced.

The `tt_call!` and `tt_return!` abstraction along with `$caller:tt` tracking
of the call hierarchy are critical to building composable macros that freely
pass around arbitrary tokens and return in a way that can inform expansion
of their caller.

A future eager expansion feature for declarative macros may render the
tt-call approach unnecessary. Eager expansion is listed as an unresolved
question in the [tracking issue for declarative macros 2.0][tracking] but is
believed to be quite a ways out, if it ever happens. And even then it is not
clear whether it is desirable to allow macros expanding to arbitrary tokens.
Today macros always expand to an expression, item, statement, type, or
pattern. Eager expansion does not automatically mean that the restriction
would be lifted to allow a macro that expands to arbitrary tokens such as `!
@ #`. The token tree calling convention provides working eager expansion
today with support for passing and returning arbitrary token streams.

[tracking]: https://github.com/rust-lang/rust/issues/39412

And function-like procedural macros once those are stable? It is going to
depend on your choice of syntax for the macro input whether a procedural
macro is a better choice, but note that they present their own DIY parsing
adventures and can be even nastier than tt-call once you get the hang of
both. In addition, procedural macros must be defined in a separate crate
from the rest of your library so they are not well suited for quick one-off
helper macros.


## Design philosphy

As may be no surprise by this point, the calling convention design
prioritizes scalability and composability over conciseness. A reader
familiar with the calling convention (maybe you, six months after writing
the macro) should be able to look at any individual tt-call rule by itself
and comfortably read off what it does top to bottom and identify its
purpose.


## Links

- The code that implements `closure!(self + 1)`, all of which is shown
  above, can be found all together in [`examples/replace.rs`].

- As a more elaborate example of a tt-call macro,
  [`examples/comma_separated.rs`] demonstrates a macro that does primitive
  name mangling of Rust types. It uses [`parse_type!`] which is a tt-call
  version of `$:ty`.

    ```rust
    static MANGLED: &[&str] = mangle_type_names! {
        std::fs::File,
        &'a mut str,
        impl Display,
        fn(s: &str) -> String,
    };

    fn main() {
        assert_eq!(MANGLED, [
            "_std_fs_File",
            "_ref_mut_str",
            "_impl_Display",
            "_fn_s_ref_str_to_String"
        ]);
    }
    ```

[`examples/replace.rs`]: https://github.com/dtolnay/tt-call/blob/master/examples/replace.rs
[`examples/comma_separated.rs`]: https://github.com/dtolnay/tt-call/blob/master/examples/comma_separated.rs
[`parse_type!`]: https://docs.rs/tt-call/1.0/tt_call/macro.parse_type.html


<br>

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in this crate by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
</sub>

[//]: # ( vim: set tw=76: )
