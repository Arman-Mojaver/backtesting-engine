//! Centralised numeric conversions.
//!
//! `clippy::as_conversions` is denied crate-wide (Cargo.toml, strict-lint
//! Phase 5), so raw `as` casts are banned from business logic. Most
//! conversions have a std `From`/`TryFrom`; the handful that do not
//! (float <-> wide integer) are routed through this module so every
//! remaining `as` cast is documented and auditable in one place.

/// Converts a `usize` to `f64`.
///
/// No std `From<usize> for f64` exists (lossy above 2^53); callers accept
/// that precision bound (see the `cast_precision_loss` allow in Cargo.toml).
#[allow(clippy::as_conversions)]
pub const fn usize_to_f64(n: usize) -> f64 {
    n as f64
}

/// Converts an `i64` to `f64`.
///
/// No std `From<i64> for f64` exists (lossy beyond the 53-bit mantissa);
/// callers accept that precision bound (see the `cast_precision_loss`
/// allow in Cargo.toml).
#[allow(clippy::as_conversions)]
pub const fn i64_to_f64(n: i64) -> f64 {
    n as f64
}

/// Converts an `i128` (e.g. an Arrow `Decimal128` value) to `f64`.
#[allow(clippy::as_conversions)]
pub const fn i128_to_f64(n: i128) -> f64 {
    n as f64
}

/// Rounds an `f64` to the nearest integer and converts to `i128`
/// (e.g. for Arrow `Decimal128` columns).
///
/// Out-of-range values saturate at the `i128` bound, which is unreachable
/// for the `Decimal128(10, 2)` columns this project writes.
#[allow(clippy::as_conversions)]
pub const fn round_f64_to_i128(x: f64) -> i128 {
    x.round() as i128
}

/// Saturating `usize -> i32` for counts (report fields, Arrow list offsets).
///
/// Out-of-range lengths clamp to `i32::MAX` instead of truncating; a count
/// overflowing two billion is not a reachable state in this codebase.
pub fn usize_to_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

/// Saturating `i32 -> usize` for periods validated non-negative upstream.
///
/// Intended for values already checked by `validate_period!`, or derived
/// from one by `saturating_sub`/integer division (which cannot go negative).
/// A negative input — only possible if validation is skipped — clamps to
/// `usize::MAX`, which every windowed computation (`windows`, `sma`, `ema`)
/// treats as insufficient data and answers with an empty result.
pub fn i32_to_usize(n: i32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}
