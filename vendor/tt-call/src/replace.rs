/// Replace each token that matches a given predicate by a given replacement
/// sequence of tokens.
/// <sup>**[tt-call]**</sup>
///
/// # Input
///
///   - `condition = [{` name of predicate macro `}]`
///   - `replace_with = [{` arbitrary tokens inserted when the predicate is true `}]`
///   - `input = [{` arbitrary input tokens `}]`
///
/// The predicate macro must accept a single input value named `input`. It is
/// expected to return a single output value which may have any name but must
/// hold the tokens `true` or `false`. For example the built-in `tt_is_ident!`
/// predicate expands to `is_ident = [{ true }]` or `is_ident = [{ false }]`.
///
/// # Output
///
///   - `tokens = [{` tokens after replacement `}]`
///
/// # Example
///
/// ```
/// use tt_call::{tt_call, tt_replace, tt_return};
///
/// macro_rules! is_lowercase_self {
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
/// macro_rules! closure {
///     ($($expr:tt)+) => {
///         |__value| tt_call! {
///             macro = [{ tt_replace }]
///             condition = [{ is_lowercase_self }]
///             replace_with = [{ __value }]
///             input = [{ $($expr)+ }]
///         }
///     };
/// }
///
/// fn main() {
///     let add_one = closure!(self + 1);
///     println!("{}", add_one(1));
/// }
/// ```
#[macro_export]
macro_rules! tt_replace {
    {
        $caller:tt
        condition = [{ $($condition:ident)::* }]
        replace_with = [{ $($with:tt)* }]
        input = [{ $($input:tt)* }]
    } => {
        $crate::private_replace! {
            $caller
            condition = [{ $($condition)::* }]
            replace_with = [{ $($with)* }]
            tokens = [{ }]
            rest = [{ $($input)* }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_replace {
    // Arrived at end of input. Return to caller.
    {
        $caller:tt
        condition = [{ $($condition:ident)::* }]
        replace_with = [{ $($with:tt)* }]
        tokens = [{ $($tokens:tt)* }]
        rest = [{ }]
    } => {
        $crate::tt_return! {
            $caller
            tokens = [{ $($tokens)* }]
        }
    };

    // Next token tree is a parenthesized group. Recurse to replace contents.
    {
        $caller:tt
        condition = [{ $($condition:ident)::* }]
        replace_with = [{ $($with:tt)* }]
        tokens = [{ $($tokens:tt)* }]
        rest = [{ ( $($group:tt)* ) $($rest:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_replace }]
            condition = [{ $($condition)::* }]
            replace_with = [{ $($with)* }]
            tokens = [{ }]
            rest = [{ $($group)* }]
            ~~> $crate::private_replace! {
                $caller
                condition = [{ $($condition)::* }]
                replace_with = [{ $($with)* }]
                tokens = [{ $($tokens)* }]
                after_paren = [{ $($rest)* }]
            }
        }
    };

    // Return from replacing contents of parenthesized group.
    {
        $caller:tt
        condition = [{ $($condition:ident)::* }]
        replace_with = [{ $($with:tt)* }]
        tokens = [{ $($tokens:tt)* }]
        after_paren = [{ $($after:tt)* }]
        tokens = [{ $($inside:tt)* }]
    } => {
        $crate::private_replace! {
            $caller
            condition = [{ $($condition)::* }]
            replace_with = [{ $($with)* }]
            tokens = [{ $($tokens)* ( $($inside)* ) }]
            rest = [{ $($after)* }]
        }
    };

    // Next token tree is a square bracketed group. Recurse to replace contents.
    {
        $caller:tt
        condition = [{ $($condition:ident)::* }]
        replace_with = [{ $($with:tt)* }]
        tokens = [{ $($tokens:tt)* }]
        rest = [{ [ $($group:tt)* ] $($rest:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_replace }]
            condition = [{ $($condition)::* }]
            replace_with = [{ $($with)* }]
            tokens = [{ }]
            rest = [{ $($group)* }]
            ~~> $crate::private_replace! {
                $caller
                condition = [{ $($condition)::* }]
                replace_with = [{ $($with)* }]
                tokens = [{ $($tokens)* }]
                after_bracket = [{ $($rest)* }]
            }
        }
    };

    // Return from replacing contents of square bracketed group.
    {
        $caller:tt
        condition = [{ $($condition:ident)::* }]
        replace_with = [{ $($with:tt)* }]
        tokens = [{ $($tokens:tt)* }]
        after_bracket = [{ $($after:tt)* }]
        tokens = [{ $($inside:tt)* }]
    } => {
        $crate::private_replace! {
            $caller
            condition = [{ $($condition)::* }]
            replace_with = [{ $($with)* }]
            tokens = [{ $($tokens)* [ $($inside)* ] }]
            rest = [{ $($after)* }]
        }
    };

    // Next token tree is a curly braced group. Recurse to replace contents.
    {
        $caller:tt
        condition = [{ $($condition:ident)::* }]
        replace_with = [{ $($with:tt)* }]
        tokens = [{ $($tokens:tt)* }]
        rest = [{ { $($group:tt)* } $($rest:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_replace }]
            condition = [{ $($condition)::* }]
            replace_with = [{ $($with)* }]
            tokens = [{ }]
            rest = [{ $($group)* }]
            ~~> $crate::private_replace! {
                $caller
                condition = [{ $($condition)::* }]
                replace_with = [{ $($with)* }]
                tokens = [{ $($tokens)* }]
                after_brace = [{ $($rest)* }]
            }
        }
    };

    // Return from replacing contents of curly braced group.
    {
        $caller:tt
        condition = [{ $($condition:ident)::* }]
        replace_with = [{ $($with:tt)* }]
        tokens = [{ $($tokens:tt)* }]
        after_brace = [{ $($after:tt)* }]
        tokens = [{ $($inside:tt)* }]
    } => {
        $crate::private_replace! {
            $caller
            condition = [{ $($condition)::* }]
            replace_with = [{ $($with)* }]
            tokens = [{ $($tokens)* { $($inside)* } }]
            rest = [{ $($after)* }]
        }
    };

    // Next token is not a group, invoke condition and continue.
    {
        $caller:tt
        condition = [{ $($condition:ident)::* }]
        replace_with = [{ $($with:tt)* }]
        tokens = [{ $($tokens:tt)* }]
        rest = [{ $first:tt $($rest:tt)* }]
    } => {
        $crate::tt_if! {
            condition = [{ $($condition)::* }]
            input = [{ $first }]
            true = [{
                $crate::private_replace! {
                    $caller
                    condition = [{ $($condition)::* }]
                    replace_with = [{ $($with)* }]
                    tokens = [{ $($tokens)* $($with)* }]
                    rest = [{ $($rest)* }]
                }
            }]
            false = [{
                $crate::private_replace! {
                    $caller
                    condition = [{ $($condition)::* }]
                    replace_with = [{ $($with)* }]
                    tokens = [{ $($tokens)* $first }]
                    rest = [{ $($rest)* }]
                }
            }]
        }
    };
}
