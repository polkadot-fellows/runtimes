/// Fail due to an unexpected input token.
///
/// The compiler's error will indicate the source of the unexpected token to the
/// user.
///
/// ```compile_fail
/// use tt_call::error_unexpected;
///
/// fn main() {
///     error_unexpected! { true }
/// }
/// ```
///
/// ```text
/// error: no rules expected the token `true`
///  --> src/unexpected.rs:5:25
///   |
/// 5 |     error_unexpected! { true }
///   |                         ^^^^
/// ```
#[macro_export]
macro_rules! error_unexpected {
    ($($tokens:tt)+) => {
        $crate::private_unexpected! {
            $($tokens)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_unexpected {
    () => {};
}

/// Fail due to an unexpected end of input.
///
/// The resulting compiler error is typically not good. Always prefer to use
/// `error_unexpected!` or `error_unexpected_last!` if there are tokens on which
/// an error could reasonably be triggered.
///
/// ```compile_fail
/// use tt_call::error_eof;
///
/// fn main() {
///     error_eof!{}
/// }
/// ```
///
/// ```text
/// error: unexpected end of macro invocation
///  --> src/unexpected.rs:5:5
///   |
/// 5 |     error_eof!{}
///   |     ^^^^^^^^^^^^
/// ```
#[macro_export]
macro_rules! error_eof {
    () => {
        $crate::private_eof! {}
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_eof {
    ($never:tt) => {};
}

/// Fail due to an unexpected input token, faulting the last token.
///
/// The compiler's error will indicate the source of the unexpected token to the
/// user.
///
/// ```compile_fail
/// use tt_call::error_unexpected_last;
///
/// fn main() {
///     error_unexpected_last! { aaa bbb ccc }
/// }
/// ```
///
/// ```text
/// error: no rules expected the token `true`
///  --> src/unexpected.rs:5:38
///   |
/// 5 |     error_unexpected_last! { aaa bbb ccc }
///   |                                      ^^^
/// ```
#[macro_export]
macro_rules! error_unexpected_last {
    ($($tokens:tt)+) => {
        $crate::private_unexpected_last! {
            $($tokens)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_unexpected_last {
    ($last:tt) => {
        $crate::error_unexpected! {
            $last
        }
    };

    ($skip:tt $($rest:tt)*) => {
        $crate::private_unexpected_last! {
            $($rest)*
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_unexpected_close_empty_square_brackets {
    ([_]) => {};
}

#[doc(hidden)]
#[macro_export]
macro_rules! private_unexpected_close_square_bracket_after_ty_semicolon {
    ([$ty:ty; _]) => {};
}
