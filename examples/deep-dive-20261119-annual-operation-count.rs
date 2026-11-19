//! Deep Dive: annual_operation_count — trades per year as a filter metric
//! Source: Code input — src/domain/annual_operation_count.rs
//!
//! A strategy that trades twice in ten years is statistically meaningless no
//! matter how good its ROI looks. This function normalises a raw trade count
//! by the backtest span (in Julian years) so GlobalStrategyFilter can reject
//! inactive strategies with one threshold, independent of how long the data is.

use std::env;

use backtesting_engine::domain::annual_operation_count::annual_operation_count;
use backtesting_engine::domain::cast::{self, METRIC_DECIMALS};
use backtesting_engine::domain::signal_group::{SignalGroup, filter_signal_groups_by_annual_ops};
use backtesting_engine::domain::strategy::GlobalStrategyFilter;
use backtesting_engine::domain::time_span::difference_in_years;

const DAY: i64 = 86_400;
// Mirrors fixtures::ONE_YEAR_SECS, which is cfg(test) and unreachable from examples.
const JULIAN_YEAR: i64 = 365 * DAY + DAY / 4;
// 2024-01-01T00:00:00Z and 2026-01-01T00:00:00Z (731 calendar days apart).
const JAN_1_2024: i64 = 1_704_067_200;
const JAN_1_2026: i64 = 1_767_225_600;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_happy_path(),
        "2" => section_2_degenerate_spans(),
        "3" => section_3_rounding(),
        "4" => section_4_filter_callers(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-annual-operation-count <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Happy Path: counts normalised by a real two-year backtest");
    println!("  2    - Degenerate Spans: zero, negative and empty inputs return 0.0");
    println!("  3    - Rounding: METRIC_DECIMALS applied before the filter compares");
    println!("  4    - Filter Callers: signal-count pre-filter vs matched-trade count");
}

// ============================================================
// SECTION 1: Happy Path - Counts Normalised by a Two-Year Backtest
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_happy_path() {
    println!("============================================================");
    println!("SECTION 1: Happy Path - Counts Normalised by a Two-Year Backtest");
    println!("============================================================");

    // 731 calendar days is slightly more than 2 Julian years (730.5 d), so the
    // result is a hair below count / 2 — rounding usually hides that.
    let years = difference_in_years(JAN_1_2024, JAN_1_2026);
    println!("span 2024-01-01 → 2026-01-01 = {years:?} Julian years");
    println!();
    println!("{:>8} | {:>12} | {:>12}", "trades", "naive /2", "annualised");
    for count in [0_usize, 1, 21, 40, 365] {
        let naive = cast::usize_to_f64(count) / 2.0;
        let result = annual_operation_count(count, JAN_1_2024, JAN_1_2026);
        println!("{count:>8} | {naive:>12.4} | {result:>12}");
    }
    println!();
    // Translation invariance: only the difference of timestamps matters.
    let shifted = annual_operation_count(40, 0, JAN_1_2026 - JAN_1_2024);
    println!("same span starting at epoch 0, 40 trades → {shifted}");
    println!("  -> start offset is irrelevant; only end - start is used");
}

// ============================================================
// SECTION 2: Degenerate Spans - Zero, Negative and Empty Inputs
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_degenerate_spans() {
    println!("============================================================");
    println!("SECTION 2: Degenerate Spans - Zero, Negative and Empty Inputs");
    println!("============================================================");

    // difference_in_years returns None for spans <= 0; the let-else turns
    // that into 0.0 so callers never see NaN or inf from a division by zero.
    let cases: [(&str, usize, i64, i64); 5] = [
        ("zero span", 10, JAN_1_2024, JAN_1_2024),
        ("negative span (end < start)", 10, JAN_1_2026, JAN_1_2024),
        ("zero count, valid span", 0, JAN_1_2024, JAN_1_2026),
        ("zero count, zero span", 0, JAN_1_2024, JAN_1_2024),
        ("one second span, 1 trade", 1, 0, 1),
    ];
    for (label, count, start, end) in cases {
        let years = difference_in_years(start, end);
        let result = annual_operation_count(count, start, end);
        println!("{label:<30} years={:<24} → {result}", format!("{years:?}"));
    }
    println!();
    println!("  -> 0.0 is indistinguishable from 'no trades': a broken span");
    println!("     simply fails any positive min_annual_operation_count");
    println!("  -> tiny spans explode (1 trade in 1 s ≈ 31.5M per year);");
    println!("     the guard only protects against <= 0, not against 'too short'");
}

// ============================================================
// SECTION 3: Rounding - METRIC_DECIMALS Before the Filter Compares
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_rounding() {
    println!("============================================================");
    println!("SECTION 3: Rounding - METRIC_DECIMALS Before the Filter Compares");
    println!("============================================================");
    println!("METRIC_DECIMALS = {METRIC_DECIMALS}");
    println!();

    let three_years = 3 * JULIAN_YEAR;
    for count in [1_usize, 2, 10] {
        let raw = cast::usize_to_f64(count) / 3.0;
        let rounded = annual_operation_count(count, 0, three_years);
        println!("{count:>3} trades / 3 years: raw={raw:<20} rounded={rounded}");
    }
    println!();

    // Rounding happens inside the metric, so the filter compares the stored
    // value — what is persisted is exactly what was judged.
    let filter = GlobalStrategyFilter::default();
    let span = JULIAN_YEAR + JULIAN_YEAR * 4 / 10_000; // 1.0004 years
    let value = annual_operation_count(10, 0, span);
    println!("10 trades over ~1.0004 years → raw ≈ 9.996, reported {value}");
    println!(
        "admits_activity(min = {}) → {}",
        filter.min_annual_operation_count,
        filter.admits_activity(value)
    );
    println!("  -> a raw 9.996 is admitted because it is rounded to 10.0 first");
}

// ============================================================
// SECTION 4: Filter Callers - Signal Count vs Matched-Trade Count
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_filter_callers() {
    println!("============================================================");
    println!("SECTION 4: Filter Callers - Signal Count vs Matched-Trade Count");
    println!("============================================================");

    // Caller 1: filter_signal_groups_by_annual_ops (domain/signal_group.rs)
    // uses raw signal counts — a cheap upper bound computed before any
    // operation-point lookup. Caller 2: create_strategies uses the count of
    // signals that actually matched an operation point, which is what is stored.
    let group = |id: i32, long: usize, short: usize| SignalGroup {
        indicator_id: id,
        long_signals: (0..long).map(|i| JAN_1_2024 + DAY * i64::try_from(i).unwrap_or(0)).collect(),
        short_signals: (0..short).map(|i| JAN_1_2024 + DAY * i64::try_from(i).unwrap_or(0)).collect(),
        instrument: "EURUSD".to_string(),
    };
    let groups = vec![group(1, 30, 25), group(2, 8, 7), group(3, 0, 0)];
    let filter = GlobalStrategyFilter::default();

    for g in &groups {
        let ops = annual_operation_count(g.signal_count(), JAN_1_2024, JAN_1_2026);
        println!("group {}: {} signals → {ops}/yr", g.indicator_id, g.signal_count());
    }
    let kept = filter_signal_groups_by_annual_ops(groups, &filter, JAN_1_2024, JAN_1_2026);
    let ids: Vec<i32> = kept.iter().map(|g| g.indicator_id).collect();
    println!("min_annual_operation_count = {}", filter.min_annual_operation_count);
    println!("kept after pre-filter: {ids:?}");
    println!();
    println!("  -> group 1 passes on 27.48 signals/yr, but if only 15 of its 55");
    println!("     signals match operation points, create_strategies recomputes");
    println!("     {} /yr and drops it in the second pass",
        annual_operation_count(15, JAN_1_2024, JAN_1_2026));
    println!("  -> the pre-filter never drops a group the second pass would keep,");
    println!("     since matched trades <= signals");
}
