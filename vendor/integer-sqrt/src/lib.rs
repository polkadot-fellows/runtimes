//!
//! This module contains the single trait [`IntegerSquareRoot`] and implements it for primitive
//! integer types.
//!
//! # Example
//!
//! ```
//! extern crate integer_sqrt;
//! // `use` trait to get functionality
//! use integer_sqrt::IntegerSquareRoot;
//!
//! # fn main() {
//! assert_eq!(4u8.integer_sqrt(), 2);
//! # }
//! ```
//!
//! [`IntegerSquareRoot`]: ./trait.IntegerSquareRoot.html
#![no_std]

/// A trait implementing integer square root.
pub trait IntegerSquareRoot {
    /// Find the integer square root.
    ///
    /// See [Integer_square_root on wikipedia][wiki_article] for more information (and also the
    /// source of this algorithm)
    ///
    /// # Panics
    ///
    /// For negative numbers (`i` family) this function will panic on negative input
    ///
    /// [wiki_article]: https://en.wikipedia.org/wiki/Integer_square_root
    fn integer_sqrt(&self) -> Self
    where
        Self: Sized,
    {
        self.integer_sqrt_checked()
            .expect("cannot calculate square root of negative number")
    }

    /// Find the integer square root, returning `None` if the number is negative (this can never
    /// happen for unsigned types).
    fn integer_sqrt_checked(&self) -> Option<Self>
    where
        Self: Sized;
}

impl<T: num_traits::PrimInt> IntegerSquareRoot for T {
    fn integer_sqrt_checked(&self) -> Option<Self> {
        use core::cmp::Ordering;
        match self.cmp(&T::zero()) {
            // Hopefully this will be stripped for unsigned numbers (impossible condition)
            Ordering::Less => return None,
            Ordering::Equal => return Some(T::zero()),
            _ => {}
        }

        // Compute bit, the largest power of 4 <= n
        let max_shift: u32 = T::zero().leading_zeros() - 1;
        let shift: u32 = (max_shift - self.leading_zeros()) & !1;
        let mut bit = T::one().unsigned_shl(shift);

        // Algorithm based on the implementation in:
        // https://en.wikipedia.org/wiki/Methods_of_computing_square_roots#Binary_numeral_system_(base_2)
        // Note that result/bit are logically unsigned (even if T is signed).
        let mut n = *self;
        let mut result = T::zero();
        while bit != T::zero() {
            if n >= (result + bit) {
                n = n - (result + bit);
                result = result.unsigned_shr(1) + bit;
            } else {
                result = result.unsigned_shr(1);
            }
            bit = bit.unsigned_shr(2);
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::IntegerSquareRoot;
    use core::{i8, u16, u64, u8};

    macro_rules! gen_tests {
        ($($type:ty => $fn_name:ident),*) => {
            $(
                #[test]
                fn $fn_name() {
                    let newton_raphson = |val, square| 0.5 * (val + (square / val as $type) as f64);
                    let max_sqrt = {
                        let square = <$type>::max_value();
                        let mut value = (square as f64).sqrt();
                        for _ in 0..2 {
                            value = newton_raphson(value, square);
                        }
                        let mut value = value as $type;
                        // make sure we are below the max value (this is how integer square
                        // root works)
                        if value.checked_mul(value).is_none() {
                            value -= 1;
                        }
                        value
                    };
                    let tests: [($type, $type); 9] = [
                        (0, 0),
                        (1, 1),
                        (2, 1),
                        (3, 1),
                        (4, 2),
                        (81, 9),
                        (80, 8),
                        (<$type>::max_value(), max_sqrt),
                        (<$type>::max_value() - 1, max_sqrt),
                    ];
                    for &(in_, out) in tests.iter() {
                        assert_eq!(in_.integer_sqrt(), out, "in {}", in_);
                    }
                }
            )*
        };
    }

    gen_tests! {
        i8 => i8_test,
        u8 => u8_test,
        i16 => i16_test,
        u16 => u16_test,
        i32 => i32_test,
        u32 => u32_test,
        i64 => i64_test,
        u64 => u64_test,
        u128 => u128_test,
        isize => isize_test,
        usize => usize_test
    }

    #[test]
    fn i128_test() {
        let tests: [(i128, i128); 8] = [
            (0, 0),
            (1, 1),
            (2, 1),
            (3, 1),
            (4, 2),
            (81, 9),
            (80, 8),
            (i128::max_value(), 13_043_817_825_332_782_212),
        ];
        for &(in_, out) in tests.iter() {
            assert_eq!(in_.integer_sqrt(), out, "in {}", in_);
        }
    }
}
