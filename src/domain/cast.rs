//! Centralised numeric conversions.
//!
//! Raw `as` casts are banned from business logic (`clippy::as_conversions`
//! is denied crate-wide). Most conversions have a std `From`/`TryFrom`; the
//! few that do not are routed through this module so every remaining cast
//! sits in one auditable place.

#[allow(clippy::as_conversions)]
pub const fn usize_to_f64(n: usize) -> f64 {
    n as f64
}

#[allow(clippy::as_conversions)]
pub const fn i64_to_f64(n: i64) -> f64 {
    n as f64
}

#[allow(clippy::as_conversions)]
pub const fn i128_to_f64(n: i128) -> f64 {
    n as f64
}

#[allow(clippy::as_conversions)]
pub const fn round_f64_to_i128(x: f64) -> i128 {
    debug_assert!(x.is_finite(), "NaN/inf would silently cast to 0/saturate");
    x.round() as i128
}

pub fn i32_to_usize(n: i32) -> usize {
    debug_assert!(n >= 0, "negative input would silently map to usize::MAX");
    usize::try_from(n).unwrap_or(usize::MAX)
}

pub fn round_to(value: f64, decimals: i32) -> f64 {
    let multiplier = 10_f64.powi(decimals);
    (value * multiplier).round() / multiplier
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_f64_to_i128_rounds_finite_values() {
        assert_eq!(round_f64_to_i128(1234.5), 1235);
    }

    #[test]
    #[should_panic(expected = "NaN/inf")]
    fn round_f64_to_i128_rejects_nan_in_debug() {
        round_f64_to_i128(f64::NAN);
    }

    #[test]
    #[should_panic(expected = "NaN/inf")]
    fn round_f64_to_i128_rejects_infinity_in_debug() {
        round_f64_to_i128(f64::INFINITY);
    }

    #[test]
    fn i32_to_usize_converts_non_negative_values() {
        assert_eq!(i32_to_usize(0), 0);
        assert_eq!(i32_to_usize(14), 14);
    }

    #[test]
    #[should_panic(expected = "negative input")]
    fn i32_to_usize_rejects_negative_in_debug() {
        i32_to_usize(-1);
    }
}
