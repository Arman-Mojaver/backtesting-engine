//! Deep Dive: difference_in_years — the annualisation denominator
//! Source: Code input — src/domain/time_span.rs
//!
//! Every "per year" metric (annual ROI, annual operation count) divides by the
//! span between the first and last timestamp expressed in Julian years. This
//! function owns that conversion and its single policy decision: a span that is
//! not strictly positive has no meaningful "years", so it returns None and the
//! callers fall back to 0.0 instead of dividing by zero.

use std::env;

use backtesting_engine::domain::annual_operation_count::annual_operation_count;
use backtesting_engine::domain::performance_metrics::PerformanceMetrics;
use backtesting_engine::domain::time_span::difference_in_years;

const DAY: i64 = 86_400;
// Mirrors SECONDS_PER_YEAR (private) and fixtures::ONE_YEAR_SECS (cfg(test)).
const JULIAN_YEAR: i64 = 365 * DAY + DAY / 4;
// 2024-01-01T00:00:00Z — a realistic Unix timestamp in seconds.
const JAN_1_2024: i64 = 1_704_067_200;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_julian_year(),
        "2" => section_2_none_guard(),
        "3" => section_3_saturating_sub(),
        "4" => section_4_units_and_precision(),
        "5" => section_5_callers(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-time-span <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Julian Year: why 365.25 days and not a calendar year");
    println!("  2    - None Guard: zero, negative and one-second spans");
    println!("  3    - Saturating Sub: extreme timestamps never overflow");
    println!("  4    - Units and Precision: seconds in, f64 years out");
    println!("  5    - Callers: how None becomes 0.0 in annual metrics");
}

// ============================================================
// SECTION 1: Julian Year - Why 365.25 Days
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_julian_year() {
    println!("============================================================");
    println!("SECTION 1: Julian Year - Why 365.25 Days");
    println!("============================================================");

    // The divisor is a fixed 365.25 * 86_400 s, independent of real calendars.
    // Averaging leap years keeps multi-year backtests unbiased without any
    // date library, at the cost of single calendar years being slightly off.
    let cases = [
        ("Julian year (365.25 d)", JULIAN_YEAR),
        ("half Julian year", JULIAN_YEAR / 2),
        ("calendar 2023 (365 d)", 365 * DAY),
        ("calendar 2024 (366 d)", 366 * DAY),
        ("4 calendar years", 3 * 365 * DAY + 366 * DAY),
        ("30 days", 30 * DAY),
    ];
    for (label, seconds) in cases {
        let years = difference_in_years(JAN_1_2024, JAN_1_2024 + seconds);
        println!("  {label:<24} {seconds:>11} s -> {years:?}");
    }
    println!();
    println!("  -> Exactly 1.0 only for the Julian year");
    println!("  -> 365 d reads 0.99932, 366 d reads 1.00205");
    println!("  -> Four calendar years (one leap) land exactly on 4.0");
}

// ============================================================
// SECTION 2: None Guard - Zero, Negative and One-Second Spans
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_none_guard() {
    println!("============================================================");
    println!("SECTION 2: None Guard - Zero, Negative and One-Second Spans");
    println!("============================================================");

    // `if seconds <= 0 { return None; }` is the only branch. It is what keeps
    // `x / years` and `powf(1.0 / years)` in the callers finite.
    let cases = [
        ("start == end", JAN_1_2024, JAN_1_2024),
        ("end before start", JAN_1_2024 + DAY, JAN_1_2024),
        ("one second", JAN_1_2024, JAN_1_2024 + 1),
        ("one day", JAN_1_2024, JAN_1_2024 + DAY),
    ];
    for (label, start, end) in cases {
        let years = difference_in_years(start, end);
        println!("  {label:<18} span={:>6} s -> {years:?}", end - start);
    }
    println!();
    println!("  -> Zero and negative spans: None (no division possible)");
    println!("  -> Arguments are ordered: (start, end), never swapped silently");
    println!("  -> Any positive span, however tiny, is Some(> 0.0)");
}

// ============================================================
// SECTION 3: Saturating Sub - Extreme Timestamps Never Overflow
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_saturating_sub() {
    println!("============================================================");
    println!("SECTION 3: Saturating Sub - Extreme Timestamps Never Overflow");
    println!("============================================================");

    // A plain `end - start` would panic in debug and wrap in release when the
    // true difference exceeds i64::MAX. saturating_sub clamps instead.
    let (start, end) = (i64::MIN, i64::MAX);
    println!("  start = i64::MIN, end = i64::MAX");
    println!("  wrapping_sub   -> {}", end.wrapping_sub(start));
    println!("  checked_sub    -> {:?}", end.checked_sub(start));
    println!("  saturating_sub -> {}", end.saturating_sub(start));
    println!();
    println!("  difference_in_years -> {:?}", difference_in_years(start, end));
    println!();
    // The reverse direction saturates to i64::MIN, still <= 0, so still None.
    println!("  reversed: saturating_sub -> {}", start.saturating_sub(end));
    println!("  reversed: difference_in_years -> {:?}", difference_in_years(end, start));
    println!();
    println!("  -> Wrapping would flip the sign to -1 and wrongly return None");
    println!("  -> Saturation keeps the sign, so the None guard stays correct");
}

// ============================================================
// SECTION 4: Units and Precision - Seconds In, f64 Years Out
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_units_and_precision() {
    println!("============================================================");
    println!("SECTION 4: Units and Precision - Seconds In, f64 Years Out");
    println!("============================================================");

    // The function trusts its unit: timestamps are Unix SECONDS. Feeding
    // milliseconds is not detected; it just scales the answer by 1000.
    let one_year_ms = JULIAN_YEAR * 1000;
    println!("  1 Julian year in seconds -> {:?}", difference_in_years(0, JULIAN_YEAR));
    println!("  1 Julian year in millis  -> {:?}", difference_in_years(0, one_year_ms));
    println!();

    // cast::i64_to_f64 is an `as` cast: exact up to 2^53 seconds (~285M years),
    // so any real backtest span converts losslessly.
    let exact_limit: i64 = 1 << 53;
    println!("  2^53 s = {:.0} years (exact f64 range)", exact_limit as f64 / JULIAN_YEAR as f64);
    println!("  2^53 + 1 as f64 == 2^53 as f64 ? {}", (exact_limit + 1) as f64 == exact_limit as f64);
    println!();
    println!("  -> Millisecond input silently yields 1000x the years");
    println!("  -> Precision loss only starts far beyond any market history");
}

// ============================================================
// SECTION 5: Callers - How None Becomes 0.0 in Annual Metrics
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_callers() {
    println!("============================================================");
    println!("SECTION 5: Callers - How None Becomes 0.0 in Annual Metrics");
    println!("============================================================");

    // Both callers use `let Some(years) = ... else { return 0.0 };`.
    let metrics = PerformanceMetrics { global_roi: 21.0, max_draw_down: 5.0 };
    let two_years = JAN_1_2024 + 2 * JULIAN_YEAR;

    println!("  annual_operation_count(120 ops)");
    println!("    over 2 years -> {}", annual_operation_count(120, JAN_1_2024, two_years));
    println!("    zero span    -> {}", annual_operation_count(120, JAN_1_2024, JAN_1_2024));
    println!();
    println!("  PerformanceMetrics {{ global_roi: 21.0 }}.annual_roi");
    println!("    over 2 years -> {}  (1.21^(1/2) - 1)", metrics.annual_roi(JAN_1_2024, two_years));
    println!("    zero span    -> {}", metrics.annual_roi(JAN_1_2024, JAN_1_2024));
    println!("    reversed     -> {}", metrics.annual_roi(two_years, JAN_1_2024));
    println!();
    println!("  -> None is never surfaced: callers map it to a 0.0 metric");
    println!("  -> 120 ops / 2.0 y = 60.0; ROI compounds via powf(1 / years)");
}
