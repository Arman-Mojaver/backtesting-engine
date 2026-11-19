//! Deep Dive: SignalGroup — one indicator's entry timestamps, and the cheap pre-filter
//! Source: Code input — src/domain/signal_group.rs
//!
//! A SignalGroup is the hand-off between signal detection and strategy creation:
//! IndicatorOutput::create_signal_group builds one per indicator, then
//! filter_signal_groups_by_annual_ops drops inactive groups before
//! create_strategies does the expensive operation-point lookups and metrics.

use std::env;

use backtesting_engine::domain::annual_operation_count::annual_operation_count;
use backtesting_engine::domain::resampled_point::ResampledPointD1;
use backtesting_engine::domain::signal::{IndicatorOutput, IndicatorValue, ThresholdBuffer};
use backtesting_engine::domain::signal_group::{SignalGroup, filter_signal_groups_by_annual_ops};
use backtesting_engine::domain::strategy::GlobalStrategyFilter;

const DAY: i64 = 86_400;
// Mirrors fixtures::ONE_YEAR_SECS, which is cfg(test) and unreachable from examples.
const JULIAN_YEAR: i64 = 365 * DAY + DAY / 4;
const JAN_1_2024: i64 = 1_704_067_200;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_anatomy(),
        "2" => section_2_signal_count(),
        "3" => section_3_filter_guards(),
        "4" => section_4_span_and_ordering(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-signal-group <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Anatomy: how an RSI-like threshold indicator becomes a SignalGroup");
    println!("  2    - signal_count: long + short, duplicates included");
    println!("  3    - Filter Guards: the count > 0 check vs admits_activity");
    println!("  4    - Span and Ordering: the same group judged over 1 vs 2 years");
}

fn day(n: i64) -> i64 {
    JAN_1_2024 + n * DAY
}

fn group(indicator_id: i32, long_count: i64, short_count: i64) -> SignalGroup {
    SignalGroup {
        indicator_id,
        long_signals: (0..long_count).map(day).collect(),
        short_signals: (0..short_count).map(day).collect(),
        instrument: "EURUSD".to_string(),
    }
}

fn ids(groups: &[SignalGroup]) -> Vec<i32> {
    groups.iter().map(|g| g.indicator_id).collect()
}

// ============================================================
// SECTION 1: Anatomy - RSI-like Threshold Indicator Becomes a SignalGroup
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_anatomy() {
    println!("============================================================");
    println!("SECTION 1: Anatomy - RSI-like Threshold Indicator Becomes a SignalGroup");
    println!("============================================================");

    // Long = crosses above the low level (30), short = crosses below the high level (70).
    let rsi = [45.0, 25.0, 35.0, 75.0, 65.0, 50.0];
    let items: Vec<IndicatorValue> = (0..)
        .zip(rsi)
        .map(|(i, value)| IndicatorValue { timestamp: day(i), value })
        .collect();
    let resampled_points: Vec<ResampledPointD1> = (0..7)
        .map(|i| ResampledPointD1 {
            id: 0, instrument: "EURUSD".to_string(), open: 1.1, high: 1.1,
            low: 1.1, close: 1.1, volume: 0, timestamp: day(i),
        })
        .collect();
    println!("RSI by day: {rsi:?}  (levels 30 / 70)");

    let output = IndicatorOutput::Threshold {
        instrument: "EURUSD".to_string(),
        buffer: ThresholdBuffer { items, high_threshold: 70.0, low_threshold: 30.0 },
    };
    let sg = output.create_signal_group(7, &resampled_points);
    let as_day = |ts: &i64| (ts - JAN_1_2024) / DAY;
    println!("indicator_id = {}, instrument = {}", sg.indicator_id, sg.instrument);
    println!("long_signals  (day) = {:?}", sg.long_signals.iter().map(as_day).collect::<Vec<_>>());
    println!("short_signals (day) = {:?}", sg.short_signals.iter().map(as_day).collect::<Vec<_>>());
    println!();
    println!("  -> cross above 30 happens on day 2, short cross below 70 on day 4,");
    println!("     but both are stored one resampled point later (day 3 / day 5):");
    println!("     the value needs day T's close, so the earliest entry is T+1's open");
    println!("  -> timestamps are what create_strategies looks up in the op-point maps");
    println!("  -> instrument is only carried on to Strategy::instrument for tracing");
}

// ============================================================
// SECTION 2: signal_count - Long Plus Short, Duplicates Included
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_signal_count() {
    println!("============================================================");
    println!("SECTION 2: signal_count - Long Plus Short, Duplicates Included");
    println!("============================================================");

    for (long, short) in [(0, 0), (5, 0), (0, 3), (4, 7)] {
        let g = group(1, long, short);
        println!("longs={long:<2} shorts={short:<2} → signal_count() = {}", g.signal_count());
    }

    // A long and a short on the same day are two signals, not one.
    let same_day = SignalGroup {
        long_signals: vec![day(3)],
        short_signals: vec![day(3)],
        ..group(2, 0, 0)
    };
    println!("long and short both on day 3 → signal_count() = {}", same_day.signal_count());
    println!();
    println!("  -> it is a pure length sum: no dedup, no lookup in operation points");
    println!("  -> saturating_add instead of + satisfies the arithmetic_side_effects lint;");
    println!("     two Vec lengths can never realistically overflow usize");
    println!("  -> const fn: callable in const contexts, required by missing_const_for_fn");
}

// ============================================================
// SECTION 3: Filter Guards - count > 0 Check vs admits_activity
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_filter_guards() {
    println!("============================================================");
    println!("SECTION 3: Filter Guards - count > 0 Check vs admits_activity");
    println!("============================================================");

    let groups = || vec![group(1, 20, 0), group(2, 6, 4), group(3, 3, 0), group(4, 0, 0)];
    for g in groups() {
        let ops = annual_operation_count(g.signal_count(), 0, JULIAN_YEAR);
        println!("group {}: {:>2} signals over 1 year → {ops}/yr", g.indicator_id, g.signal_count());
    }
    println!();

    for min in [10.0, 0.0] {
        let filter = GlobalStrategyFilter { min_annual_operation_count: min, ..GlobalStrategyFilter::default() };
        let kept = filter_signal_groups_by_annual_ops(groups(), &filter, 0, JULIAN_YEAR);
        println!("min_annual_operation_count = {min:<4} → kept {:?}", ids(&kept));
    }
    println!();
    println!("  -> group 2 passes on 6 long + 4 short: both sides count together");
    println!("  -> at min 0.0, admits_activity(0.0) alone would keep group 4;");
    println!("     the signal_count() > 0 guard is what drops a group with no signals");
    println!("  -> && short-circuits, so empty groups never compute an annual rate");
}

// ============================================================
// SECTION 4: Span and Ordering - Same Group Over 1 vs 2 Years
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_span_and_ordering() {
    println!("============================================================");
    println!("SECTION 4: Span and Ordering - Same Group Over 1 vs 2 Years");
    println!("============================================================");

    let filter = GlobalStrategyFilter::default();
    let groups = || vec![group(4, 12, 0), group(1, 15, 0), group(9, 25, 0)];
    println!("input order: {:?}, min = {}", ids(&groups()), filter.min_annual_operation_count);

    for (label, end) in [("1 year ", JULIAN_YEAR), ("2 years", 2 * JULIAN_YEAR), ("0 span ", 0)] {
        let kept = filter_signal_groups_by_annual_ops(groups(), &filter, 0, end);
        println!("{label} → kept {:?}", ids(&kept));
    }
    println!();
    println!("  -> 15 signals is 15/yr over one year but 7.5/yr over two: the");
    println!("     span comes from the operation points, not from the signals");
    println!("  -> a zero span makes annual_operation_count return 0.0, so");
    println!("     everything fails a positive minimum");
    println!("  -> takes Vec by value and returns survivors in input order;");
    println!("     no clone of the signal vectors is made");
    println!();
    println!("  -> second pass (create_strategies) re-checks with matched trades only;");
    println!("     see deep-dive-20261119-annual-operation-count section 4");
}
