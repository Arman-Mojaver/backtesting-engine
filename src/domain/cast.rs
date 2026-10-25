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
    x.round() as i128
}

pub fn usize_to_i32(n: usize) -> i32 {
    i32::try_from(n).unwrap_or(i32::MAX)
}

pub fn i32_to_usize(n: i32) -> usize {
    usize::try_from(n).unwrap_or(usize::MAX)
}

pub fn round_to(value: f64, decimals: i32) -> f64 {
    let multiplier = 10_f64.powi(decimals);
    (value * multiplier).round() / multiplier
}
