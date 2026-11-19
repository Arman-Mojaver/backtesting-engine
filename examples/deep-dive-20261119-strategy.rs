//! Deep Dive: Strategy + GlobalStrategyFilter
//! Source: Code input — src/domain/strategy.rs
//!
//! `Strategy` is the stored result of one (indicator, mms) backtest: its
//! headline metrics plus the operation point ids that produced them.
//! `GlobalStrategyFilter` is the project-wide quality bar a candidate must
//! clear — once cheaply on activity, then on performance — before it is kept.

use std::env;

use backtesting_engine::domain::annual_operation_count::annual_operation_count;
use backtesting_engine::domain::signal_group::{SignalGroup, filter_signal_groups_by_annual_ops};
use backtesting_engine::domain::strategy::{GlobalStrategyFilter, Strategy};

// Mirrors the test-only fixture: a Julian year, matching time_span::SECONDS_PER_YEAR.
const ONE_YEAR_SECS: i64 = 365 * 86_400 + 86_400 / 4;
const START: i64 = 1_600_000_000;
const TWO_YEARS_LATER: i64 = START + 2 * ONE_YEAR_SECS;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_strategy_anatomy(),
        "2" => section_2_default_filter(),
        "3" => section_3_admits_activity(),
        "4" => section_4_admits_performance(),
        "5" => section_5_nan_metrics(),
        "6" => section_6_two_stage_activity_check(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-strategy <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Strategy Anatomy: what one stored result holds");
    println!("  2    - Default Filter: the baseline quality bar and overrides");
    println!("  3    - Admits Activity: inclusive minimum on annual operation count");
    println!("  4    - Admits Performance: ROI floor AND draw-down ceiling");
    println!("  5    - NaN Metrics: why a broken metric is always rejected");
    println!("  6    - Two-Stage Activity Check: raw signals, then matched trades");
}

fn sample_strategy() -> Strategy {
    Strategy {
        instrument: "EURUSD".into(),
        indicator_id: 4217,
        annual_roi: 12.37,
        max_draw_down: 8.41,
        annual_operation_count: 14.5,
        long_operation_point_ids: vec![101, 117, 133, 160],
        short_operation_point_ids: vec![108, 142, 171],
        money_management_strategy_id: 390,
    }
}

// ============================================================
// SECTION 1: Strategy Anatomy - What One Stored Result Holds
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_strategy_anatomy() {
    println!("============================================================");
    println!("SECTION 1: Strategy Anatomy - What One Stored Result Holds");
    println!("============================================================");

    let s = sample_strategy();
    // Identity: which indicator under which money-management strategy.
    println!("identity : indicator_id={} mms_id={}", s.indicator_id, s.money_management_strategy_id);
    // Metrics: what the filter judges and what gets stored.
    println!("metrics  : annual_roi={} max_draw_down={} annual_ops={}",
        s.annual_roi, s.max_draw_down, s.annual_operation_count);
    // Evidence: the exact trades, so a stored strategy can be recomputed.
    println!("long ids : {:?}", s.long_operation_point_ids);
    println!("short ids: {:?}", s.short_operation_point_ids);
    println!();

    // `instrument` is never read by the engine and is not a Parquet column:
    // the partition path instrument=EURUSD/... already encodes it.
    println!("instrument (tracing only): {}", s.instrument);
    println!();
    println!("  -> No constructor, no invariants: create_strategy in");
    println!("     src/application/strategy_generator.rs fills every field.");
    println!("  -> Clone is deep (Strings + Vecs); Debug is derived for dumps.");
}

// ============================================================
// SECTION 2: Default Filter - The Baseline Quality Bar and Overrides
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_default_filter() {
    println!("============================================================");
    println!("SECTION 2: Default Filter - The Baseline Quality Bar and Overrides");
    println!("============================================================");

    // strategy_runner.rs uses exactly this — there is no config override path.
    let filter = GlobalStrategyFilter::default();
    println!("{filter:?}");
    println!();
    println!("  max_draw_down_max         = {:>5}  (ceiling, inclusive)", filter.max_draw_down_max);
    println!("  min_annual_roi            = {:>5}  (floor, inclusive)", filter.min_annual_roi);
    println!("  min_annual_operation_count= {:>5}  (floor, inclusive)", filter.min_annual_operation_count);
    println!();

    // `max_draw_down_max`: draw_down_max is the metric (max of the series);
    // the leading max_ is the global limit on it. Two different "max".

    // Struct-update syntax, as the tests' min_ops() helper does.
    let strict = GlobalStrategyFilter { min_annual_operation_count: 50.0, ..GlobalStrategyFilter::default() };
    println!("override: {strict:?}");
    println!("  -> Only the named field changes; the rest stays at the defaults.");
}

// ============================================================
// SECTION 3: Admits Activity - Inclusive Minimum on Annual Operation Count
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_admits_activity() {
    println!("============================================================");
    println!("SECTION 3: Admits Activity - Inclusive Minimum on Annual Operation Count");
    println!("============================================================");

    let filter = GlobalStrategyFilter::default();
    // The filter never sees raw counts: callers annualize over the data span first.
    println!("span: 2 Julian years; min = {}/yr", filter.min_annual_operation_count);
    for trades in [0_usize, 19, 20, 21, 60] {
        let per_year = annual_operation_count(trades, START, TWO_YEARS_LATER);
        println!("  {trades:>2} trades -> {per_year:>5}/yr -> admits_activity = {}",
            filter.admits_activity(per_year));
    }
    println!();

    // Edge: a zero/negative span makes annual_operation_count return 0.0,
    // which then fails any positive minimum — no division by zero.
    let collapsed = annual_operation_count(500, START, START);
    println!("500 trades over 0s -> {collapsed}/yr -> {}", filter.admits_activity(collapsed));
    println!();
    println!("  -> `>=`: exactly 10.0/yr passes (20 trades / 2 years).");
}

// ============================================================
// SECTION 4: Admits Performance - ROI Floor AND Draw-Down Ceiling
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_admits_performance() {
    println!("============================================================");
    println!("SECTION 4: Admits Performance - ROI Floor AND Draw-Down Ceiling");
    println!("============================================================");

    let filter = GlobalStrategyFilter::default();
    println!("rule: annual_roi >= {} && max_draw_down <= {}", filter.min_annual_roi, filter.max_draw_down_max);
    println!();
    // (annual_roi, max_draw_down, why)
    let cases = [
        (12.37, 8.41, "comfortably inside both bounds"),
        (5.0, 10.0, "ROI exactly at floor (inclusive)"),
        (4.99, 10.0, "ROI just below floor"),
        (15.0, 20.0, "draw-down exactly at ceiling (inclusive)"),
        (15.0, 20.01, "draw-down just above ceiling"),
        (80.0, 35.0, "huge ROI cannot buy back a deep draw-down"),
        (-3.0, 2.0, "tiny draw-down cannot rescue a losing strategy"),
    ];
    for (roi, dd, why) in cases {
        let verdict = if filter.admits_performance(roi, dd) { "KEEP" } else { "drop" };
        println!("  roi={roi:>6} dd={dd:>6} -> {verdict}  ({why})");
    }
    println!();
    println!("  -> AND, not a score: each bound is a hard veto.");
    println!("  -> Applied last in create_strategies, after the costly metrics.");
}

// ============================================================
// SECTION 5: NaN Metrics - Why a Broken Metric Is Always Rejected
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_nan_metrics() {
    println!("============================================================");
    println!("SECTION 5: NaN Metrics - Why a Broken Metric Is Always Rejected");
    println!("============================================================");

    let filter = GlobalStrategyFilter::default();
    // Every comparison with NaN is false. Both checks are written as
    // "value passes when <comparison> is true", so NaN fails closed.
    let nan = std::hint::black_box(f64::NAN);
    println!("NaN >= 5.0  -> {}", nan >= 5.0);
    println!("NaN <= 20.0 -> {}", nan <= 20.0);
    println!();
    println!("admits_performance(NaN, 10.0)  -> {}", filter.admits_performance(f64::NAN, 10.0));
    println!("admits_performance(15.0, NaN)  -> {}", filter.admits_performance(15.0, f64::NAN));
    println!("admits_activity(NaN)           -> {}", filter.admits_activity(f64::NAN));
    println!();

    // Contrast: a negated form like !(dd > max) would let NaN through.
    let dd = f64::NAN;
    println!("hypothetical !(dd > 20.0)      -> {}  (would fail OPEN)", !(dd > 20.0));
    println!();
    // Infinities are ordinary ordered values, so they follow the bounds.
    println!("admits_performance(inf, 10.0)  -> {}", filter.admits_performance(f64::INFINITY, 10.0));
    println!("admits_performance(15.0, inf)  -> {}", filter.admits_performance(15.0, f64::INFINITY));
}

// ============================================================
// SECTION 6: Two-Stage Activity Check - Raw Signals, Then Matched Trades
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_two_stage_activity_check() {
    println!("============================================================");
    println!("SECTION 6: Two-Stage Activity Check - Raw Signals, Then Matched Trades");
    println!("============================================================");

    let filter = GlobalStrategyFilter::default();
    let group = |id: i32, longs: i64, shorts: i64| SignalGroup {
        indicator_id: id,
        long_signals: (0..longs).map(|i| START + i * 3_600).collect(),
        short_signals: (0..shorts).map(|i| START + i * 7_200).collect(),
        instrument: "EURUSD".into(),
    };
    let groups = vec![group(1, 14, 10), group(2, 6, 4), group(3, 0, 0)];

    // Stage 1 (signal_group.rs): raw signal count. Free, and an upper bound:
    // a signal can only become a trade if an operation point exists for it.
    for g in &groups {
        let per_year = annual_operation_count(g.signal_count(), START, TWO_YEARS_LATER);
        println!("stage 1: indicator {} -> {:>2} signals -> {per_year:>4}/yr", g.indicator_id, g.signal_count());
    }
    let survivors = filter_signal_groups_by_annual_ops(groups, &filter, START, TWO_YEARS_LATER);
    println!("  survivors: {:?}", survivors.iter().map(|g| g.indicator_id).collect::<Vec<_>>());
    println!();

    // Stage 2 (strategy_generator.rs): only signals that hit an operation
    // point count. Suppose 6 of indicator 1's signals fall in the ATR warm-up
    // or tail, where no operation point exists:
    let matched = annual_operation_count(24 - 6, START, TWO_YEARS_LATER);
    println!("stage 2: indicator 1 -> 18 matched trades -> {matched}/yr -> {}", filter.admits_activity(matched));
    println!();
    println!("  -> Stage 1 never drops a group stage 2 would keep (matched <= raw).");
    println!("  -> Stage 2 runs before performance_metrics, the expensive step.");
}
