// The code below is based on code from the statrs library.
// See LICENSE-THIRD-PARTY for details.

use std::f64;

use crate::ulps_eq;

#[cfg(test)]
mod tests;

// ln(pi)
const LN_PI: f64 = 1.144_729_885_849_400_2;
// ln(2 * sqrt(e / pi))
const LN_2_SQRT_E_OVER_PI: f64 = 0.620_782_237_635_245_2;
// Auxiliary variable when evaluating the `gamma_ln` function
const GAMMA_R: f64 = 10.900511;

// Polynomial coefficients for approximating the `gamma_ln` function
const GAMMA_DK: [f64; 11] = [
    2.485_740_891_387_535_5e-5,
    1.051_423_785_817_219_7,
    -3.456_870_972_220_162_5,
    4.512_277_094_668_948,
    -2.982_852_253_235_766_4,
    1.056_397_115_771_267,
    -1.954_287_731_916_458_7e-1,
    1.709_705_434_044_412e-2,
    -5.719_261_174_043_057e-4,
    4.633_994_733_599_057e-6,
    -2.719_949_084_886_077_2e-9,
];

// Standard epsilon, maximum relative precision of IEEE 754 double-precision
// floating point numbers (64 bit) e.g. `2^-53`
const F64_PREC: f64 = 1.1102230246251565e-16;

/// Calculates the cumulative distribution function for the student's at `x`
/// with location `0` and scale `1`.
///
/// # Formula
///
/// ```ignore
/// if x < μ {
///     (1 / 2) * I(t, v / 2, 1 / 2)
/// } else {
///     1 - (1 / 2) * I(t, v / 2, 1 / 2)
/// }
/// ```
///
/// where `t = v / (v + k^2)`, `k = (x - μ) / σ`, `μ` is the location,
/// `σ` is the scale, `v` is the freedom, and `I` is the regularized
/// incomplete
/// beta function
pub fn students_t_cdf(x: f64, freedom: i64) -> Option<f64> {
    if freedom <= 0 {
        return None;
    }
    let location: f64 = 0.;
    let scale: f64 = 1.0;
    let freedom = freedom as f64;
    let k = (x - location) / scale;
    let h = freedom / (freedom + k * k);
    let ib = 0.5 * checked_beta_reg(freedom / 2.0, 0.5, h)?;
    if x <= location {
        Some(ib)
    } else {
        Some(1.0 - ib)
    }
}

/// Computes the regularized lower incomplete beta function
/// `I_x(a,b) = 1/Beta(a,b) * int(t^(a-1)*(1-t)^(b-1), t=0..x)`
/// `a > 0`, `b > 0`, `1 >= x >= 0` where `a` is the first beta parameter,
/// `b` is the second beta parameter, and `x` is the upper limit of the
/// integral.
///
/// Returns `None` if `a <= 0.0`, `b <= 0.0`, `x < 0.0`, or `x > 1.0`
fn checked_beta_reg(a: f64, b: f64, x: f64) -> Option<f64> {
    if a <= 0. || b <= 0. || !(0.0..=1.).contains(&x) {
        return None;
    }
    let bt = if x == 0. || ulps_eq(x, 1.0, f64::EPSILON, 4) {
        0.0
    } else {
        (ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln()).exp()
    };
    let symm_transform = x >= (a + 1.0) / (a + b + 2.0);
    let eps = F64_PREC;
    let fpmin = f64::MIN_POSITIVE / eps;

    let mut a = a;
    let mut b = b;
    let mut x = x;
    if symm_transform {
        let swap = a;
        x = 1.0 - x;
        a = b;
        b = swap;
    }

    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;

    if d.abs() < fpmin {
        d = fpmin;
    }
    d = 1.0 / d;
    let mut h = d;

    for m in 1..141 {
        let m = f64::from(m);
        let m2 = m * 2.0;
        let mut aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;

        if d.abs() < fpmin {
            d = fpmin;
        }

        c = 1.0 + aa / c;
        if c.abs() < fpmin {
            c = fpmin;
        }

        d = 1.0 / d;
        h = h * d * c;
        aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;

        if d.abs() < fpmin {
            d = fpmin;
        }

        c = 1.0 + aa / c;

        if c.abs() < fpmin {
            c = fpmin;
        }

        d = 1.0 / d;
        let del = d * c;
        h *= del;

        if (del - 1.0).abs() <= eps {
            return if symm_transform {
                Some(1.0 - bt * h / a)
            } else {
                Some(bt * h / a)
            };
        }
    }

    if symm_transform {
        Some(1.0 - bt * h / a)
    } else {
        Some(bt * h / a)
    }
}

/// Computes the logarithm of the gamma function
/// with an accuracy of 16 floating point digits.
/// The implementation is derived from
/// "An Analysis of the Lanczos Gamma Approximation",
/// Glendon Ralph Pugh, 2004 p. 116
fn ln_gamma(x: f64) -> f64 {
    if x < 0.5 {
        let s = GAMMA_DK
            .iter()
            .enumerate()
            .skip(1)
            .fold(GAMMA_DK[0], |s, t| s + t.1 / (t.0 as f64 - x));

        LN_PI
            - (f64::consts::PI * x).sin().ln()
            - s.ln()
            - LN_2_SQRT_E_OVER_PI
            - (0.5 - x) * ((0.5 - x + GAMMA_R) / f64::consts::E).ln()
    } else {
        let s = GAMMA_DK
            .iter()
            .enumerate()
            .skip(1)
            .fold(GAMMA_DK[0], |s, t| s + t.1 / (x + t.0 as f64 - 1.0));

        s.ln() + LN_2_SQRT_E_OVER_PI + (x - 0.5) * ((x - 0.5 + GAMMA_R) / f64::consts::E).ln()
    }
}
