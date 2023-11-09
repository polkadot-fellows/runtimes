//! This example demonstrates use of `tt_replace!` to implement a fanciful
//! syntax for unary closures.

use tt_call::{tt_call, tt_replace, tt_return};

/// Predicate that determines whether the input is the token `self`.
/// <sup>**[tt-call]**</sup>
///
/// # Input
///
///   - `input = [{` any single token `}]`
///
/// # Output
///
///   - `is = [{` true or false `}]`
macro_rules! is_lowercase_self {
    {
        $caller:tt
        input = [{ self }]
    } => {
        tt_return! {
            $caller
            is = [{ true }]
        }
    };

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

/// Expands to a closure with one argument called `self`. For example
/// `closure!(self + 1)` would expand to `|__value| __value + 1`.
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
