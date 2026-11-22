//! Deep Dive: src/domain/cast.rs
//!
//! Centralised numeric conversions and rounding. Raw `as` casts are banned
//! from business logic (clippy::as_conversions is denied crate-wide), so every
//! remaining cast sits in this one auditable module. Decimal rounding of
//! reported metrics lives here too, so every caller uses the same rule.
//!
//! Run:   cargo run --example deep-dive-20261117-cast <section>
//! Cleanup: rm examples/deep-dive-*.rs

use backtesting_engine::domain::cast::{
    METRIC_DECIMALS, i128_to_f64, i32_to_usize, i64_to_f64, round_f64_to_i128, round_to,
    usize_to_f64,
};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let section = args.get(1).map(|s| s.as_str()).unwrap_or("list");
    match section {
        "1" => section_1_int_to_f64(),
        "2" => section_2_round_half_away(),
        "3" => section_3_saturation(),
        "4" => section_4_i32_to_usize(),
        "5" => section_5_metric_rounding(),
        "6" => section_6_binary_gotchas(),
        "list" | _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261117-cast <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Integer to f64: Three Const Casts, One Precision Trap");
    println!("  2    - round_f64_to_i128: Halves Round Away From Zero");
    println!("  3    - round_f64_to_i128: Saturation at i128 Bounds");
    println!("  4    - i32_to_usize: Conversion and the Negative-Input Trap");
    println!("  5    - round_to: The Single Metric Rounding Rule");
    println!("  6    - round_to: Binary Representation Gotchas");
}

// ============================================================
// SECTION 1: Integer to f64 - Three Const Casts, One Precision Trap
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_int_to_f64() {
    println!("============================================================");
    println!("SECTION 1: Integer to f64 - Three Const Casts, One Precision Trap");
    println!("============================================================");
    println!("usize_to_f64(42)              = {}", usize_to_f64(42));
    println!("i64_to_f64(-42)               = {}", i64_to_f64(-42));
    println!("i128_to_f64(7)                = {}", i128_to_f64(7));
    println!("i64_to_f64(i64::MAX)          = {}", i64_to_f64(i64::MAX));
    println!("i64_to_f64(i64::MAX - 1)      = {}", i64_to_f64(i64::MAX - 1));
    println!("i128_to_f64(i128::MAX)        = {}", i128_to_f64(i128::MAX));
    println!();
    println!("  -> f64 has a 53-bit mantissa; i64/i128 hold up to 64/128 bits.");
    println!("  -> Adjacent big integers collapse to the SAME f64 (see MAX vs MAX-1).");
    println!("  -> All three are `pub const fn`: usable in const context, zero cost.");
}

// ============================================================
// SECTION 2: round_f64_to_i128 - Halves Round Away From Zero
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_round_half_away() {
    println!("============================================================");
    println!("SECTION 2: round_f64_to_i128 - Halves Round Away From Zero");
    println!("============================================================");
    println!("round_f64_to_i128(1234.5)      = {}", round_f64_to_i128(1234.5));
    println!("round_f64_to_i128(2.5)         = {}   <- half rounds AWAY from zero", round_f64_to_i128(2.5));
    println!("round_f64_to_i128(-2.5)        = {}  <- same for negatives", round_f64_to_i128(-2.5));
    println!("round_f64_to_i128(0.4)         = {}", round_f64_to_i128(0.4));
    println!("round_f64_to_i128(-0.4)        = {}   <- |x| < 0.5 rounds toward zero", round_f64_to_i128(-0.4));
    println!("round_f64_to_i128(10_000_000.7) = {}", round_f64_to_i128(10_000_000.7));
    println!();
    println!("  -> Uses f64::round(): half-away-from-zero (NOT banker's rounding).");
    println!("  -> Why i128 output? Integer timestamps/counts can exceed i64 today.");
}

// ============================================================
// SECTION 3: round_f64_to_i128 - Saturation at i128 Bounds
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_saturation() {
    println!("============================================================");
    println!("SECTION 3: round_f64_to_i128 - Saturation at i128 Bounds");
    println!("============================================================");
    println!("i128 range is roughly +/-1.7e38. Input 1e40 exceeds it:");
    println!("  round_f64_to_i128(1e40)  = {}", round_f64_to_i128(1e40));
    println!("  round_f64_to_i128(-1e40) = {}", round_f64_to_i128(-1e40));
    println!();
    println!("  -> Since Rust 1.45 float->int casts SATURATE (never UB).");
    println!("  -> Guard: debug_assert!(x.is_finite()) - see src/domain/cast.rs:26");
    println!("  -> Debug builds: NaN/inf input panics with 'NaN/inf would silently");
    println!("     cast to 0/saturate'. Release builds: assert gone, NaN casts to 0.");
    println!("  -> Tests lock this in: cast.rs:58-73 (saturate, reject NaN, reject inf).");
}

// ============================================================
// SECTION 4: i32_to_usize - Conversion and the Negative-Input Trap
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_i32_to_usize() {
    println!("============================================================");
    println!("SECTION 4: i32_to_usize - Conversion and the Negative-Input Trap");
    println!("============================================================");
    println!("i32_to_usize(0)           = {}", i32_to_usize(0));
    println!("i32_to_usize(14)          = {}", i32_to_usize(14));
    println!("i32_to_usize(i32::MAX)    = {}", i32_to_usize(i32::MAX));
    println!();
    println!("  -> Body: usize::try_from(n).unwrap_or(usize::MAX)  (cast.rs:30-33)");
    println!("  -> try_from fails ONLY for negative n; unwrap_or maps that to usize::MAX.");
    println!("  -> debug_assert!(n >= 0): debug builds panic on negative input;");
    println!("     in release the assert vanishes and -1 silently becomes usize::MAX");
    println!("     (a 20-digit index - a nasty bug if it reaches a slice).");
    println!("  -> Why not `i32 as usize`? Ambiguous/lossy - clippy bans it crate-wide.");
}

// ============================================================
// SECTION 5: round_to - The Single Metric Rounding Rule
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_metric_rounding() {
    println!("============================================================");
    println!("SECTION 5: round_to - The Single Metric Rounding Rule");
    println!("============================================================");
    println!("METRIC_DECIMALS = {}  (pub const, cast.rs:35)", METRIC_DECIMALS);
    println!("round_to(12.345678, METRIC_DECIMALS) = {}", round_to(12.345678, METRIC_DECIMALS));
    println!("round_to(12.345678, 0)   = {}", round_to(12.345678, 0));
    println!("round_to(-12.346, 2)     = {}", round_to(-12.346, 2));
    println!("round_to(0.125, 2)       = {}", round_to(0.125, 2));
    println!();
    println!("  -> Formula: (value * 10^decimals).round() / 10^decimals");
    println!("  -> Same half-away-from-zero rule as f64::round().");
    println!("  -> One rule for every reported metric: no ad-hoc rounding in callers.");
}

// ============================================================
// SECTION 6: round_to - Binary Representation Gotchas
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_binary_gotchas() {
    println!("============================================================");
    println!("SECTION 6: round_to - Binary Representation Gotchas");
    println!("============================================================");
    println!("round_to(1.005, 2)  = {}   <- surprise! rounds DOWN", round_to(1.005, 2));
    println!("round_to(0.125, 2)  = {}  <- 0.125 is exact in binary, rounds up", round_to(0.125, 2));
    println!("round_to(f64::NAN, 2) = {}  (propagates silently)", round_to(f64::NAN, 2));
    println!();
    println!("  -> 1.005 is stored as 1.00499999999999989..., so .round() floors it.");
    println!("  -> f64 stores decimals in binary; most decimal literals are inexact.");
    println!("  -> Tests document this behaviour: cast.rs:105-109.");
    println!("  -> Rule: round at the END of a calculation, never mid-pipeline, and");
    println!("     avoid == comparisons on rounded values.");
}
