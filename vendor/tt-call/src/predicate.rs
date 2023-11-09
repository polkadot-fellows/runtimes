/// Predicate that is always true.
/// <sup>**[tt-call]**</sup>
///
/// # Input
///
///   - `input = [{` anything `}]`
///
/// # Output
///
///   - `output = [{ true }]`
#[macro_export]
macro_rules! tt_true {
    {
        $caller:tt
        input = [{ $($in:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            output = [{ true }]
        }
    }
}

/// Predicate that is always false.
/// <sup>**[tt-call]**</sup>
///
/// # Input
///
///   - `input = [{` anything `}]`
///
/// # Output
///
///   - `output = [{ false }]`
#[macro_export]
macro_rules! tt_false {
    {
        $caller:tt
        input = [{ $($in:tt)* }]
    } => {
        $crate::tt_return! {
            $caller
            output = [{ false }]
        }
    }
}

/// Predicate that accepts a single token and determines whether it is a comma.
/// <sup>**[tt-call]**</sup>
///
/// # Input
///
///   - `input = [{` a single token tree `}]`
///
/// # Output
///
///   - `is_comma = [{` either true or false `}]`
#[macro_export]
macro_rules! tt_is_comma {
    {
        $caller:tt
        input = [{ , }]
    } => {
        $crate::tt_return! {
            $caller
            is_comma = [{ true }]
        }
    };

    {
        $caller:tt
        input = [{ $other:tt }]
    } => {
        $crate::tt_return! {
            $caller
            is_comma = [{ false }]
        }
    };
}

/// Predicate that accepts a single token and determines whether it is an
/// identifier.
/// <sup>**[tt-call]**</sup>
///
/// An identifier is anything that matches Rust's `$:ident` fragment.
///
/// # Input
///
///   - `input = [{` a single token tree `}]`
///
/// # Output
///
///   - `is_ident = [{` either true or false `}]`
#[macro_export]
macro_rules! tt_is_ident {
    {
        $caller:tt
        input = [{ $ident:ident }]
    } => {
        $crate::tt_return! {
            $caller
            is_ident = [{ true }]
        }
    };

    {
        $caller:tt
        input = [{ $other:tt }]
    } => {
        $crate::tt_return! {
            $caller
            is_ident = [{ false }]
        }
    };
}

/// Predicate that accepts a single token and determines whether it is a
/// lifetime token.
/// <sup>**[tt-call]**</sup>
///
/// # Input
///
///   - `input = [{` a single token tree `}]`
///
/// # Output
///
///   - `is_lifetime = [{` either true or false `}]`
#[macro_export]
macro_rules! tt_is_lifetime {
    {
        $caller:tt
        input = [{ $lifetime:lifetime }]
    } => {
        $crate::tt_return! {
            $caller
            is_lifetime = [{ true }]
        }
    };

    {
        $caller:tt
        input = [{ $other:tt }]
    } => {
        $crate::tt_return! {
            $caller
            is_lifetime = [{ false }]
        }
    };
}
