/// Parse any syntactically valid Rust type.
/// <sup>**[tt-call]**</sup>
///
/// This is the tt-call equivalent of Rust's `$:ty` fragment.
///
/// # Input
///
///   - `input = [{` tokens `}]`
///
/// # Output
///
///   - `type = [{` tokens of type `}]`
///   - `rest = [{` remaining tokens after type `}]`
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
macro_rules! parse_type {
    {
        $caller:tt
        input = [{ $($tt:tt)* }]
    } => {
        $crate::private_parse_type! {
            $caller
            tokens = [{ $($tt)* }]
            _tokens = [{ $($tt)* }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_type_with_plus {
    // Entry point.
    {
        $caller:tt
        input = [{ $($input:tt)* }]
    } => {
        $crate::private_parse_type_with_plus! {
            $caller
            pieces = [{ }]
            tokens = [{ $($input)* }]
        }
    };

    // There is at least one previously parsed piece, and next token is a
    // lifetime.
    {
        $caller:tt
        pieces = [{ $($pieces:tt)+ }]
        tokens = [{ $lifetime:lifetime $($rest:tt)* }]
    } => {
        $crate::private_parse_type_with_plus! {
            $caller
            pieces = [{ $($pieces)* }]
            type = [{ $lifetime }]
            rest = [{ $($rest)* }]
        }
    };

    // Next token is not a lifetime or this is the first piece. Parse as a type.
    {
        $caller:tt
        pieces = [{ $($pieces:tt)* }]
        tokens = [{ $($tokens:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($tokens)* }]
            ~~> $crate::private_parse_type_with_plus! {
                $caller
                pieces = [{ $($pieces)* }]
            }
        }
    };

    // Return from parse_type. Dup the rest tokens.
    {
        $caller:tt
        pieces = [{ $($pieces:tt)* }]
        type = [{ $($ty:tt)* }]
        rest = [{ $($rest:tt)* }]
    } => {
        $crate::private_parse_type_with_plus! {
            $caller
            pieces = [{ $($pieces)* }]
            type = [{ $($ty)* }]
            rest = [{ $($rest)* }]
            _rest = [{ $($rest)* }]
        }
    };

    // Most recently parsed type or lifetime is followed by a plus. Recurse.
    {
        $caller:tt
        pieces = [{ $($pieces:tt)* }]
        type = [{ $($ty:tt)* }]
        rest = [{ + $($rest:tt)* }]
        _rest = [{ $plus:tt $($dup:tt)* }]
    } => {
        $crate::private_parse_type_with_plus! {
            $caller
            pieces = [{ $($pieces)* $($ty)* $plus }]
            tokens = [{ $($rest)* }]
        }
    };

    // Not followed by a plus so the plus-separated type is done. Return.
    {
        $caller:tt
        pieces = [{ $($pieces:tt)* }]
        type = [{ $($ty:tt)* }]
        rest = [{ $($rest:tt)* }]
        _rest = [{ $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $($pieces)* $($ty)* }]
            rest = [{ $($rest)* }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_type {
    // First token is a nonempty bracketed group. Validate and return as slice
    // or array.
    {
        $caller:tt
        tokens = [{ [$($bracketed:tt)+] $($rest:tt)* }]
        _tokens = [{ $original:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_validate_inside_brackets }]
            original = [{ $original }]
            input = [{ $($bracketed)* }]
            ~~> $crate::tt_return! {
                $caller
                type = [{ $original }]
                rest = [{ $($rest)* }]
            }
        }
    };

    // Unexpected: first token is empty brackets
    {
        $caller:tt
        tokens = [{ [] $($rest:tt)* }]
        _tokens = [{ $first:tt $($dup:tt)* }]
    } => {
        $crate::private_unexpected_close_empty_square_brackets! {
            $first
        }
    };

    // First token is a parenthesized group. Validate and return as tuple type.
    {
        $caller:tt
        tokens = [{ ($($parenthesized:tt)*) $($rest:tt)* }]
        _tokens = [{ $original:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_validate_inside_parens }]
            input = [{ $($parenthesized)* }]
            ~~> $crate::tt_return! {
                $caller
                type = [{ $original }]
                rest = [{ $($rest)* }]
            }
        }
    };

    // First token is asterisk. Parse a pointer.
    {
        $caller:tt
        tokens = [{ * $($rest:tt)+ }]
        _tokens = [{ $asterisk:tt $($dup:tt)* }]
    } => {
        $crate::private_parse_pointer! {
            $caller
            pointer = [{ $asterisk }]
            tokens = [{ $($rest)* }]
        }
    };

    // First token is ampersand. Parse a reference.
    {
        $caller:tt
        tokens = [{ & $($rest:tt)+ }]
        _tokens = [{ $ampersand:tt $($dup:tt)* }]
    } => {
        $crate::private_parse_reference! {
            $caller
            reference = [{ $ampersand }]
            tokens = [{ $($rest)* }]
        }
    };

    // First token is `fn` keyword. Parse a function pointer type.
    {
        $caller:tt
        tokens = [{ fn $($rest:tt)+ }]
        _tokens = [{ $fn:tt $($dup:tt)* }]
    } => {
        $crate::private_parse_function! {
            $caller
            function = [{ $fn }]
            tokens = [{ $($rest)* }]
        }
    };

    // Unexpected: input ends with `fn` keyword.
    {
        $caller:tt
        tokens = [{ fn }]
        _tokens = [{ $unexpected:tt }]
    } => {
        $crate::error_unexpected! {
            $unexpected
        }
    };

    // The never type. Return.
    {
        $caller:tt
        tokens = [{ ! $($rest:tt)* }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ ! }]
            rest = [{ $($rest)* }]
        }
    };

    // First token is `dyn` contextual keyword. Parse trait type with plus.
    {
        $caller:tt
        tokens = [{ dyn $($rest:tt)+ }]
        _tokens = [{ $dyn:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_type_with_plus }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_type! {
                $caller
                object = [{ $dyn }]
            }
        }
    };

    // Unexpected: input ends with `dyn` contextual keyword.
    {
        $caller:tt
        tokens = [{ dyn }]
        _tokens = [{ $unexpected:tt }]
    } => {
        $crate::error_unexpected! {
            $unexpected
        }
    };

    // First token is `impl` keyword. Parse trait type with plus.
    {
        $caller:tt
        tokens = [{ impl $($rest:tt)+ }]
        _tokens = [{ $impl:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_type_with_plus }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_type! {
                $caller
                object = [{ $impl }]
            }
        }
    };

    // Unexpected: input ends with `impl` keyword.
    {
        $caller:tt
        tokens = [{ impl }]
        _tokens = [{ $unexpected:tt }]
    } => {
        $crate::error_unexpected! {
            $unexpected
        }
    };

    // Return from parsing type after `dyn` or `impl`.
    {
        $caller:tt
        object = [{ $kind:ident }]
        type = [{ $($element:tt)* }]
        rest = [{ $($rest:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $kind $($element)* }]
            rest = [{ $($rest)* }]
        }
    };

    // The underscore inferred type.
    {
        $caller:tt
        tokens = [{ _ $($rest:tt)* }]
        _tokens = [{ $underscore:tt $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $underscore }]
            rest = [{ $($rest)* }]
        }
    };

    // First token is `for` keyword. Parse poly trait.
    {
        $caller:tt
        tokens = [{ for $($rest:tt)+ }]
        _tokens = [{ $for:tt $($dup:tt)* }]
    } => {
        $crate::private_parse_poly_trait! {
            $caller
            poly_trait = [{ $for }]
            tokens = [{ $($rest)* }]
        }
    };

    // Unexpected: input ends with `for` keyword.
    {
        $caller:tt
        tokens = [{ for }]
        _tokens = [{ $unexpected:tt }]
    } => {
        $crate::error_unexpected! {
            $unexpected
        }
    };

    // Type macro invocation with relative path and parentheses.
    //
    // TODO: preserve the span of colons, bang, and parens.
    {
        $caller:tt
        tokens = [{ $($path:ident)::+ ! ( $($args:tt)* ) $($rest:tt)* }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $($path)::* ! ( $($args)* ) }]
            rest = [{ $($rest)* }]
        }
    };

    // Type macro invocation with absolute path and parentheses.
    {
        $caller:tt
        tokens = [{ $(:: $path:ident)+ ! ( $($args:tt)* ) $($rest:tt)* }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $(:: $path)* ! ( $($args)* ) }]
            rest = [{ $($rest)* }]
        }
    };

    // Type macro invocation with relative path and square brackets.
    {
        $caller:tt
        tokens = [{ $($path:ident)::+ ! [ $($args:tt)* ] $($rest:tt)* }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $($path)::* ! [ $($args)* ] }]
            rest = [{ $($rest)* }]
        }
    };

    // Type macro invocation with absolute path and square brackets.
    {
        $caller:tt
        tokens = [{ $(:: $path:ident)+ ! [ $($args:tt)* ] $($rest:tt)* }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $(:: $path)* ! [ $($args)* ] }]
            rest = [{ $($rest)* }]
        }
    };

    // Type macro invocation with relative path and curly braces.
    {
        $caller:tt
        tokens = [{ $($path:ident)+ ! { $($args:tt)* } $($rest:tt)* }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $($path)::* ! { $($args)* } }]
            rest = [{ $($rest)* }]
        }
    };

    // Type macro invocation with absolute path and curly braces.
    {
        $caller:tt
        tokens = [{ $(:: $path:ident)+ ! { $($args:tt)* } $($rest:tt)* }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $(:: $path)* ! { $($args)* } }]
            rest = [{ $($rest)* }]
        }
    };

    // First token is open angle bracket qualified path.
    {
        $caller:tt
        tokens = [{ < $($rest:tt)+ }]
        _tokens = [{ $lt:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_type! {
                $caller
                type_prefix = [{ $lt }]
            }
        }
    };

    // Unexpected: input ends with open angle bracket.
    {
        $caller:tt
        type_prefix = [{ < }]
        type = [{ $($ty:tt)+ }]
        rest = [{ }]
    } => {
        $crate::error_unexpected_last! {
            $($ty)*
        }
    };

    // Return from parsing angle bracketed path prefix element. Dup the rest
    // tokens.
    {
        $caller:tt
        type_prefix = [{ $($prefix:tt)* }]
        type = [{ $($ty:tt)* }]
        rest = [{ $($rest:tt)+ }]
    } => {
        $crate::private_parse_type! {
            $caller
            type_prefix = [{ $($prefix)* }]
            type = [{ $($ty)* }]
            rest = [{ $($rest)* }]
            _rest = [{ $($rest)* }]
        }
    };

    // Angle brackets are fully qualified trait syntax with absolute path.
    {
        $caller:tt
        type_prefix = [{ $lt:tt }]
        type = [{ $($qself:tt)* }]
        rest = [{ as :: $_segment:ident $($rest:tt)* }]
        _rest = [{ $as:tt $colons:tt $segment:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_possibly_empty_path_after_ident }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_type! {
                $caller
                qpath = [{ $lt $($qself)* $as $colons $segment }]
            }
        }
    };

    // Unexpected: angle bracketed trait absolute path is invalid.
    {
        $caller:tt
        type_prefix = [{ $lt:tt }]
        type = [{ $($qself:tt)* }]
        rest = [{ as :: $($unexpected:tt)+ }]
        _rest = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Angle brackets are fully qualified trait syntax with relative path.
    {
        $caller:tt
        type_prefix = [{ $lt:tt }]
        type = [{ $($qself:tt)* }]
        rest = [{ as $_segment:ident $($rest:tt)* }]
        _rest = [{ $as:tt $segment:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_possibly_empty_path_after_ident }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_type! {
                $caller
                qpath = [{ $lt $($qself)* $as $segment }]
            }
        }
    };

    // Unexpected: angle bracketed trait relative path is invalid.
    {
        $caller:tt
        type_prefix = [{ < }]
        type = [{ $($qself:tt)* }]
        rest = [{ as $($unexpected:tt)+ }]
        _rest = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Return from parsing fully qualified trait syntax. Dup the rest tokens.
    {
        $caller:tt
        qpath = [{ $($qpath:tt)* }]
        path = [{ $($path:tt)* }]
        rest = [{ $($rest:tt)* }]
    } => {
        $crate::private_parse_type! {
            $caller
            qpath = [{ $($qpath)* }]
            path = [{ $($path)* }]
            rest = [{ $($rest)* }]
            _rest = [{ $($rest)* }]
        }
    };

    // Close angle bracket of fully qualified trait syntax. Parse rest of path.
    {
        $caller:tt
        qpath = [{ $($qpath:tt)* }]
        path = [{ $($path:tt)* }]
        rest = [{ > :: $_segment:ident $($rest:tt)* }]
        _rest = [{ $gt:tt $colons:tt $segment:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_possibly_empty_path_after_ident }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_type! {
                $caller
                path_prefix = [{ $($qpath)* $($path)* $gt $colons $segment }]
            }
        }
    };

    // Unexpected: path after fully qualified trait syntax is invalid.
    {
        $caller:tt
        qpath = [{ $($qpath:tt)* }]
        path = [{ $($path:tt)* }]
        rest = [{ > :: $($unexpected:tt)+ }]
        _rest = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Unexpected: fully qualified trait syntax is not followed by path.
    {
        $caller:tt
        qpath = [{ $($qpath:tt)* }]
        path = [{ $($path:tt)* }]
        rest = [{ > $($unexpected:tt)+ }]
        _rest = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Unexpected: failed to find close angle bracket of fully qualified trait
    // syntax.
    {
        $caller:tt
        qpath = [{ $($qpath:tt)* }]
        path = [{ $($path:tt)* }]
        rest = [{ $($unexpected:tt)+ }]
        _rest = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Unexpected: input ends inside of angle bracketed trait path.
    {
        $caller:tt
        qpath = [{ $($qpath:tt)* }]
        path = [{ $($path:tt)* }]
        rest = [{ }]
        _rest = [{ }]
    } => {
        $crate::error_unexpected_last! {
            $($qpath)* $($path)*
        }
    };

    // Angle brackets are a type by itself, not trait path. Parse rest of path.
    {
        $caller:tt
        type_prefix = [{ $lt:tt }]
        type = [{ $($qself:tt)* }]
        rest = [{ > :: $_segment:ident $($rest:tt)* }]
        _rest = [{ $gt:tt $colons:tt $segment:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_possibly_empty_path_after_ident }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_type! {
                $caller
                path_prefix = [{ $lt $($qself)* $gt $colons $segment }]
            }
        }
    };

    // Unexpected: failed to find close angle bracket after type.
    {
        $caller:tt
        type_prefix = [{ < }]
        type = [{ $($qself:tt)* }]
        rest = [{ $($unexpected:tt)+ }]
        _rest = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Parse absolute path.
    {
        $caller:tt
        tokens = [{ :: $_segment:ident $($rest:tt)* }]
        _tokens = [{ $colons:tt $segment:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_path }]
            input = [{ $colons $segment $($rest)* }]
            ~~> $crate::private_parse_type! {
                $caller
                path_prefix = [{ }]
            }
        }
    };

    // Unexpected: invalid start of absolute path.
    {
        $caller:tt
        tokens = [{ :: $($unexpected:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Parse relative path.
    {
        $caller:tt
        tokens = [{ $_segment:ident $($rest:tt)* }]
        _tokens = [{ $segment:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_path }]
            input = [{ $segment $($rest)* }]
            ~~> $crate::private_parse_type! {
                $caller
                path_prefix = [{ }]
            }
        }
    };

    // First token is `?` for a maybe-trait.
    {
        $caller:tt
        tokens = [{ ? $($tokens:tt)* }]
        _tokens = [{ $question:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_path }]
            input = [{ $($tokens)* }]
            ~~> $crate::private_parse_type! {
                $caller
                path_prefix = [{ $question }]
            }
        }
    };

    // Return from parsing a path.
    {
        $caller:tt
        path_prefix = [{ $($prefix:tt)* }]
        path = [{ $($path:tt)* }]
        rest = [{ $($rest:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $($prefix)* $($path)* }]
            rest = [{ $($rest)* }]
        }
    };

    // Unexpected: unrecognized first token.
    {
        $caller:tt
        tokens = [{ $($unexpected:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Unexpected: input is empty.
    {
        $caller:tt
        tokens = [{ }]
        _tokens = [{ }]
    } => {
        $crate::error_eof! {}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_pointer {
    // Entry point. Dup tokens.
    {
        $caller:tt
        pointer = [{ $($pointer:tt)* }]
        tokens = [{ $($tokens:tt)* }]
    } => {
        $crate::private_parse_pointer! {
            $caller
            pointer = [{ $($pointer)* }]
            tokens = [{ $($tokens)* }]
            _tokens = [{ $($tokens)* }]
        }
    };

    // Pointer is a *const. Parse element type.
    {
        $caller:tt
        pointer = [{ $asterisk:tt }]
        tokens = [{ const $($rest:tt)+ }]
        _tokens = [{ $const:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_pointer! {
                $caller
                pointer = [{ $asterisk $const }]
            }
        }
    };

    // Pointer is a *mut. Parse element type.
    {
        $caller:tt
        pointer = [{ $asterisk:tt }]
        tokens = [{ mut $($rest:tt)+ }]
        _tokens = [{ $mut:tt $($dup:tt)+ }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_pointer! {
                $caller
                pointer = [{ $asterisk $mut }]
            }
        }
    };

    // Unexpected: unrecognized pointer type.
    {
        $caller:tt
        pointer = [{ * }]
        tokens = [{ $($unexpected:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Return from parsing element type.
    {
        $caller:tt
        pointer = [{ $($pointer:tt)* }]
        type = [{ $($element:tt)* }]
        rest = [{ $($rest:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $($pointer)* $($element)* }]
            rest = [{ $($rest)* }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_reference {
    // Entry point. Dup tokens.
    {
        $caller:tt
        reference = [{ $($reference:tt)* }]
        tokens = [{ $($tokens:tt)* }]
    } => {
        $crate::private_parse_reference! {
            $caller
            reference = [{ $($reference)* }]
            tokens = [{ $($tokens)* }]
            _tokens = [{ $($tokens)* }]
        }
    };

    // Reference has an explicit lifetime.
    {
        $caller:tt
        reference = [{ $ampersand:tt }]
        tokens = [{ $lifetime:lifetime $($rest:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::private_parse_reference! {
            $caller
            reference = [{ $ampersand $lifetime }]
            tokens = [{ $($rest)* }]
            _tokens = [{ $($rest)* }]
        }
    };

    // Exclusive reference.
    {
        $caller:tt
        reference = [{ $ampersand:tt $($lifetime:lifetime)* }]
        tokens = [{ mut $($rest:tt)+ }]
        _tokens = [{ $mut:tt $($dup:tt)* }]
    } => {
        $crate::private_parse_reference! {
            $caller
            reference = [{ $ampersand $($lifetime)* $mut }]
            tokens = [{ $($rest)* }]
            _tokens = [{ $($rest)* }]
        }
    };

    // Unexpected: input ends with &mut.
    {
        $caller:tt
        reference = [{ & $($lifetime:lifetime)* }]
        tokens = [{ mut }]
        _tokens = [{ $unexpected:tt }]
    } => {
        $crate::error_unexpected! {
            $unexpected
        }
    };

    // Parse element type.
    {
        $caller:tt
        reference = [{ $ampersand:tt $($lifetime:lifetime)* $($mut:ident)* }]
        tokens = [{ $($rest:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_reference! {
                $caller
                reference = [{ $ampersand $($lifetime)* $($mut)* }]
            }
        }
    };

    // Return from parsing element type.
    {
        $caller:tt
        reference = [{ $($reference:tt)* }]
        type = [{ $($element:tt)* }]
        rest = [{ $($rest:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $($reference)* $($element)* }]
            rest = [{ $($rest)* }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_function {
    // Entry point. Dup tokens.
    {
        $caller:tt
        function = [{ $($function:tt)* }]
        tokens = [{ $($tokens:tt)* }]
    } => {
        $crate::private_parse_function! {
            $caller
            function = [{ $($function)* }]
            tokens = [{ $($tokens)* }]
            _tokens = [{ $($tokens)* }]
        }
    };

    // Validate parenthesized function arguments.
    {
        $caller:tt
        function = [{ $fn:tt }]
        tokens = [{ ($($args:tt)*) $($rest:tt)* }]
        _tokens = [{ $paren:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_validate_fn_args }]
            input = [{ $($args)* }]
            ~~> $crate::private_parse_function! {
                $caller
                function = [{ $fn $paren }]
                rest = [{ $($rest)* }]
                _rest = [{ $($rest)* }]
            }
        }
    };

    // Unexpected: failed to find parenthesized function arguments.
    {
        $caller:tt
        function = [{ $fn:tt }]
        tokens = [{ $($unexpected:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Unexpected: input ends with `fn(...) ->`.
    {
        $caller:tt
        function = [{ $fn:tt $args:tt }]
        rest = [{ -> }]
        _rest = [{ $arrow:tt }]
    } => {
        $crate::error_unexpected! {
            $arrow
        }
    };

    // Parse function return type.
    {
        $caller:tt
        function = [{ $fn:tt $args:tt }]
        rest = [{ -> $($rest:tt)+ }]
        _rest = [{ $arrow:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_function! {
                $caller
                function = [{ $fn $args $arrow }]
            }
        }
    };

    // Function has implicit unit return type.
    {
        $caller:tt
        function = [{ $fn:tt $args:tt }]
        rest = [{ $($rest:tt)* }]
        _rest = [{ $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $fn $args }]
            rest = [{ $($rest)* }]
        }
    };

    // Return from parsing function return type.
    {
        $caller:tt
        function = [{ $fn:tt $args:tt $arrow:tt }]
        type = [{ $($ret:tt)* }]
        rest = [{ $($rest:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $fn $args $arrow $($ret)* }]
            rest = [{ $($rest)* }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_poly_trait {
    // Entry point. Dup tokens.
    {
        $caller:tt
        poly_trait = [{ $($poly_trait:tt)* }]
        tokens = [{ $($tokens:tt)+ }]
    } => {
        $crate::private_parse_poly_trait! {
            $caller
            poly_trait = [{ $($poly_trait)* }]
            tokens = [{ $($tokens)* }]
            _tokens = [{ $($tokens)* }]
        }
    };

    // Parse angle bracketed lifetimes of poly trait.
    {
        $caller:tt
        poly_trait = [{ $for:tt }]
        tokens = [{ < $($rest:tt)+ }]
        _tokens = [{ $lt:tt $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_lifetime_params }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_poly_trait! {
                $caller
                poly_trait = [{ $for $lt }]
            }
        }
    };

    // Unexpected: failed to find angle bracketed lifetimes.
    {
        $caller:tt
        poly_trait = [{ $for:tt }]
        tokens = [{ $($unexpected:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Return from parsing angle bracketed lifetimes. Parse rest of type.
    {
        $caller:tt
        poly_trait = [{ $for:tt $lt:tt }]
        lifetime_params = [{ $($params:tt)* }]
        rest = [{ $gt:tt $($rest:tt)+ }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($rest)* }]
            ~~> $crate::private_parse_poly_trait! {
                $caller
                poly_trait = [{ $for $lt $($params)* $gt }]
            }
        }
    };

    // Unexpected: input ends with `for<...>`.
    {
        $caller:tt
        poly_trait = [{ $for:tt $lt:tt }]
        lifetime_params = [{ $($params:tt)* }]
        rest = [{ $gt:tt }]
    } => {
        $crate::error_unexpected! {
            $gt
        }
    };

    // Return from parsing complete poly trait type.
    {
        $caller:tt
        poly_trait = [{ $($params:tt)* }]
        type = [{ $($ty:tt)* }]
        rest = [{ $($rest:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            type = [{ $($params)* $($ty)* }]
            rest = [{ $($rest)* }]
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_parse_lifetime_params {
    // Entry point after parsing `<`. Dup tokens.
    {
        $caller:tt
        input = [{ $($input:tt)* }]
    } => {
        $crate::private_parse_lifetime_params! {
            $caller
            lifetime_params = [{ }]
            mode = [{ < }]
            tokens = [{ $($input)* }]
            _tokens = [{ $($input)* }]
        }
    };

    // Found `>`. Return lifetimes.
    {
        $caller:tt
        lifetime_params = [{ $($params:tt)* }]
        mode = [{ $any:tt }]
        tokens = [{ > $($rest:tt)* }]
        _tokens = [{ $gt:tt $($dup:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            lifetime_params = [{ $($params)* }]
            rest = [{ $gt $($rest)* }]
        }
    };

    // Found lifetime.
    {
        $caller:tt
        lifetime_params = [{ $($params:tt)* }]
        mode = [{ < }]
        tokens = [{ $lifetime:lifetime $($rest:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::private_parse_lifetime_params! {
            $caller
            lifetime_params = [{ $($params)* $lifetime }]
            mode = [{ 'a }]
            tokens = [{ $($rest)* }]
            _tokens = [{ $($rest)* }]
        }
    };

    // Found colon after lifetime.
    {
        $caller:tt
        lifetime_params = [{ $($params:tt)* }]
        mode = [{ 'a }]
        tokens = [{ : $($rest:tt)+ }]
        _tokens = [{ $colon:tt $($dup:tt)* }]
    } => {
        $crate::private_parse_lifetime_params! {
            $caller
            lifetime_params = [{ $($params)* $colon }]
            mode = [{ : }]
            tokens = [{ $($rest)* }]
            _tokens = [{ $($rest)* }]
        }
    };

    // Found comma after lifetime.
    {
        $caller:tt
        lifetime_params = [{ $($params:tt)* }]
        mode = [{ 'a }]
        tokens = [{ , $($rest:tt)+ }]
        _tokens = [{ $comma:tt $($dup:tt)* }]
    } => {
        $crate::private_parse_lifetime_params! {
            $caller
            lifetime_params = [{ $($params)* $comma }]
            mode = [{ < }]
            tokens = [{ $($rest)* }]
            _tokens = [{ $($rest)* }]
        }
    };

    // Expect comma after colon, because lifetime bounds cannot be used in poly
    // trait.
    {
        $caller:tt
        lifetime_params = [{ $($params:tt)* }]
        mode = [{ : }]
        tokens = [{ , $($rest:tt)+ }]
        _tokens = [{ $comma:tt $($dup:tt)* }]
    } => {
        $crate::private_parse_lifetime_params! {
            $caller
            lifetime_params = [{ $($params)* $comma }]
            mode = [{ < }]
            tokens = [{ $($rest)* }]
            _tokens = [{ $($rest)* }]
        }
    };

    // Unexpected: any other token.
    {
        $caller:tt
        lifetime_params = [{ $($params:tt)* }]
        mode = [{ $any:tt }]
        tokens = [{ $($unexpected:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_validate_inside_brackets {
    // Entry point. Parse content of brackets as type.
    {
        $caller:tt
        original = [{ $original:tt }]
        input = [{ $($input:tt)+ }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($input)* }]
            ~~> $crate::private_validate_inside_brackets! {
                $caller
                original = [{ $original }]
            }
        }
    };

    // Brackets contain only a type. Valid slice type.
    {
        $caller:tt
        original = [{ $original:tt }]
        type = [{ $($bracketed:tt)* }]
        rest = [{ }]
    } => {
        $crate::tt_return! {
            $caller
        }
    };

    // Bracketed tokens are followed by semicolon. Parse expression.
    {
        $caller:tt
        original = [{ $original:tt }]
        type = [{ $($element:tt)* }]
        rest = [{ ; $($len:tt)+ }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_expr }]
            input = [{ $($len)* }]
            ~~> $crate::private_validate_inside_brackets! {
                $caller
            }
        }
    };

    // Unexpected: bracketed tokens end with semicolon: `[... ; ]`.
    {
        $caller:tt
        original = [{ $original:tt }]
        type = [{ $($element:tt)* }]
        rest = [{ ; }]
    } => {
        $crate::private_unexpected_close_square_bracket_after_ty_semicolon! {
            $original
        }
    };

    // Unexpected: type is followed by something other than a semicolon.
    {
        $caller:tt
        original = [{ $original:tt }]
        type = [{ $($element:tt)* }]
        rest = [{ $($unexpected:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };

    // Brackets contain a type, semicolon, and expression. Valid array type.
    {
        $caller:tt
        expr = [{ $($len:tt)* }]
        rest = [{ }]
    } => {
        $crate::tt_return! {
            $caller
        }
    };

    // Unexpected: remaining tokens after array length expression.
    {
        $caller:tt
        expr = [{ $($len:tt)* }]
        rest = [{ $($unexpected:tt)+ }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_validate_inside_parens {
    // Entry point. Parentheses are empty.
    {
        $caller:tt
        input = [{ }]
    } => {
        $crate::tt_return! {
            $caller
        }
    };

    // Entry point. Parse the first element as a type allowing plus.
    {
        $caller:tt
        input = [{ $($tokens:tt)+ }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::private_parse_type_with_plus }]
            input = [{ $($tokens)* }]
            ~~> $crate::private_validate_inside_parens! {
                $caller
            }
        }
    };

    // Parentheses contain valid types.
    {
        $caller:tt
        type = [{ $($element:tt)* }]
        rest = [{ }]
    } => {
        $crate::tt_return! {
            $caller
        }
    };

    // Parentheses contain valid types with trailing comma.
    {
        $caller:tt
        type = [{ $($element:tt)* }]
        rest = [{ , }]
    } => {
        $crate::tt_return! {
            $caller
        }
    };

    // Parse the next type after comma.
    {
        $caller:tt
        type = [{ $($element:tt)* }]
        rest = [{ , $($rest:tt)+ }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($rest)* }]
            ~~> $crate::private_validate_inside_parens! {
                $caller
            }
        }
    };

    // Unexpected: parenthesized type is not followed by comma.
    {
        $caller:tt
        type = [{ $($element:tt)* }]
        rest = [{ $($unexpected:tt)* }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_validate_fn_args {
    // Entry point. Dup tokens.
    {
        $caller:tt
        input = [{ $($input:tt)* }]
    } => {
        $crate::private_validate_fn_args! {
            $caller
            tokens = [{ $($input)* }]
            _tokens = [{ $($input)* }]
        }
    };

    // Function arguments are valid.
    {
        $caller:tt
        tokens = [{ }]
        _tokens = [{ }]
    } => {
        $crate::tt_return! {
            $caller
        }
    };

    // Parse type of an underscore named argument.
    {
        $caller:tt
        tokens = [{ _ : $($rest:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($rest)* }]
            ~~> $crate::private_validate_fn_args! {
                $caller
            }
        }
    };

    // Unexpected: end of arguments after underscore colon.
    {
        $caller:tt
        tokens = [{ _ : }]
        _tokens = [{ $skip:tt $colon:tt }]
    } => {
        $crate::error_unexpected! {
            $colon
        }
    };

    // Parse type of a named argument.
    {
        $caller:tt
        tokens = [{ $name:ident : $($rest:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($rest)* }]
            ~~> $crate::private_validate_fn_args! {
                $caller
            }
        }
    };

    // Unexpected: end of arguments after ident colon.
    {
        $caller:tt
        tokens = [{ $name:ident : }]
        _tokens = [{ $skip:tt $colon:tt }]
    } => {
        $crate::error_unexpected! {
            $colon
        }
    };

    // Parse type of an unnamed argument.
    {
        $caller:tt
        tokens = [{ $($rest:tt)+ }]
        _tokens = [{ $($dup:tt)* }]
    } => {
        $crate::tt_call! {
            macro = [{ $crate::parse_type }]
            input = [{ $($rest)* }]
            ~~> $crate::private_validate_fn_args! {
                $caller
            }
        }
    };

    // Validated last function argument.
    {
        $caller:tt
        type = [{ $($ty:tt)* }]
        rest = [{ }]
    } => {
        $crate::tt_return! {
            $caller
        }
    };

    // Validate next argument after comma.
    {
        $caller:tt
        type = [{ $($ty:tt)* }]
        rest = [{ , $($rest:tt)* }]
    } => {
        $crate::private_validate_fn_args! {
            $caller
            input = [{ $($rest)* }]
        }
    };

    // Unexpected: function argument is followed by something other than comma.
    {
        $caller:tt
        type = [{ $($ty:tt)* }]
        rest = [{ $($unexpected:tt)+ }]
    } => {
        $crate::error_unexpected! {
            $($unexpected)*
        }
    };
}
