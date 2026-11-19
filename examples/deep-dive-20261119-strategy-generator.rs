//! Deep Dive: create_strategies (strategy generator)
//! Source: Code input — src/application/strategy_generator.rs
//!
//! Turns surviving SignalGroups (one per indicator) into Strategy rows for a
//! single (instrument, mms) cell. Each signal timestamp is looked up in the
//! long/short operation point maps; only signals that hit a precomputed trade
//! count. Pipeline per group:
//!   matched-trade activity filter -> create_strategy (metrics) -> performance filter
//!
//! `matched_trade_count` and `create_strategy` are private, so sections 2-3
//! rebuild their steps from the same public building blocks.

use std::env;

use backtesting_engine::application::strategy_generator::create_strategies;
use backtesting_engine::domain::annual_operation_count::annual_operation_count;
use backtesting_engine::domain::operation_point::{OperationPoint, OperationPoints, points_by_timestamp};
use backtesting_engine::domain::performance_metrics::performance_metrics;
use backtesting_engine::domain::signal_group::SignalGroup;
use backtesting_engine::domain::strategy::{GlobalStrategyFilter, Strategy};

// Julian year, matching time_span::SECONDS_PER_YEAR.
const ONE_YEAR_SECS: i64 = 365 * 86_400 + 86_400 / 4;
const START: i64 = 1_600_000_000;
const END: i64 = START + ONE_YEAR_SECS;
const DAY: i64 = 86_400;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_happy_path(),
        "2" => section_2_inside_create_strategy(),
        "3" => section_3_unmatched_signals(),
        "4" => section_4_long_short_isolation(),
        "5" => section_5_three_gates(),
        "6" => section_6_edge_cases(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-strategy-generator <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Happy Path: one signal group becomes one Strategy");
    println!("  2    - Inside create_strategy: resolve, count, measure");
    println!("  3    - Unmatched Signals: warm-up and tail bars do not count");
    println!("  4    - Long/Short Isolation: same timestamp, separate maps");
    println!("  5    - Three Gates: activity, then metrics, then performance");
    println!("  6    - Edge Cases: no groups, nothing resolves, zero span");
}

// Same shape as the test fixture domain::fixtures::op_point (tp 30, sl 20, risk 2%).
fn op(id: i32, result: i32, timestamp: i64) -> OperationPoint {
    OperationPoint {
        id,
        instrument: "EURUSD".into(),
        money_management_strategy_id: 390,
        timestamp,
        result,
        tp: 30,
        sl: 20,
        risk: 0.02,
    }
}

fn group(indicator_id: i32, long_signals: Vec<i64>, short_signals: Vec<i64>) -> SignalGroup {
    SignalGroup { indicator_id, long_signals, short_signals, instrument: "EURUSD".into() }
}

// Default ROI/draw-down bars; activity lowered so a handful of trades can pass.
fn demo_filter() -> GlobalStrategyFilter {
    GlobalStrategyFilter { min_annual_operation_count: 3.0, ..GlobalStrategyFilter::default() }
}

// Longs win +60 pips on days 1 and 3; the short loses -20 on day 2.
fn seed_ops() -> (Vec<OperationPoint>, Vec<OperationPoint>) {
    let longs = vec![op(11, 60, START + DAY), op(13, 60, START + 3 * DAY)];
    let shorts = vec![op(12, -20, START + 2 * DAY)];
    (longs, shorts)
}

fn print_strategy(s: &Strategy) {
    println!("  indicator_id={} mms_id={} instrument={}", s.indicator_id, s.money_management_strategy_id, s.instrument);
    println!("  annual_roi={} max_draw_down={} annual_ops={}", s.annual_roi, s.max_draw_down, s.annual_operation_count);
    println!("  long ids={:?} short ids={:?}", s.long_operation_point_ids, s.short_operation_point_ids);
}

// ============================================================
// SECTION 1: Happy Path - One Signal Group Becomes One Strategy
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_happy_path() {
    println!("============================================================");
    println!("SECTION 1: Happy Path - One Signal Group Becomes One Strategy");
    println!("============================================================");

    let (longs, shorts) = seed_ops();
    let ops = OperationPoints { long_operation_points: &longs, short_operation_points: &shorts };
    let sg = group(4217, vec![START + DAY, START + 3 * DAY], vec![START + 2 * DAY]);
    println!("signals: long={:?}", sg.long_signals);
    println!("         short={:?}", sg.short_signals);

    // In strategy_runner, START/END come from ops.start_end_timestamps().
    let strategies = create_strategies(&[sg], &ops, &demo_filter(), 390, START, END);
    println!("create_strategies -> {} strategy", strategies.len());
    for s in &strategies {
        print_strategy(s);
    }
    println!();
    println!("  -> Ids are operation point ids, not timestamps: the stored");
    println!("     evidence for recomputing the metrics later.");
    println!("  -> mms_id is passed in, not read from the points: the caller");
    println!("     already loaded exactly one (instrument, mms) cell.");
}

// ============================================================
// SECTION 2: Inside create_strategy - Resolve, Count, Measure
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_inside_create_strategy() {
    println!("============================================================");
    println!("SECTION 2: Inside create_strategy - Resolve, Count, Measure");
    println!("============================================================");

    let (longs, shorts) = seed_ops();
    // Built ONCE per create_strategies call, shared read-only by every rayon task.
    let long_map = points_by_timestamp(&longs);
    let short_map = points_by_timestamp(&shorts);
    println!("step 0: long_map keys={} short_map keys={}", long_map.len(), short_map.len());

    // Step 1: filter_map(get) — unknown timestamps vanish silently.
    let long_ops: Vec<&OperationPoint> =
        [START + DAY, START + 3 * DAY].iter().filter_map(|ts| long_map.get(ts).copied()).collect();
    let short_ops: Vec<&OperationPoint> =
        [START + 2 * DAY].iter().filter_map(|ts| short_map.get(ts).copied()).collect();
    println!("step 1: resolved long ids={:?} short ids={:?}",
        long_ops.iter().map(|o| o.id).collect::<Vec<_>>(), short_ops.iter().map(|o| o.id).collect::<Vec<_>>());

    // Step 2: annualize the RESOLVED count, not the raw signal count.
    let count = long_ops.len() + short_ops.len();
    println!("step 2: {count} trades over 1 year -> {}/yr", annual_operation_count(count, START, END));

    // Step 3: longs and shorts merge into ONE equity curve; performance_metrics
    // sorts by timestamp, so the chain order (all longs, then shorts) is irrelevant.
    let all: Vec<&OperationPoint> = long_ops.into_iter().chain(short_ops).collect();
    let metrics = performance_metrics(all);
    println!("step 3: {metrics:?}");
    println!("        equity 1.00 -> 1.06 (+60) -> 1.0388 (-20) -> 1.1024 (+60)");

    // Step 4: compound the global ROI down to a per-year figure.
    println!("step 4: annual_roi over 1 year = {}", metrics.annual_roi(START, END));
    println!("        annual_roi over 2 years = {}", metrics.annual_roi(START, START + 2 * ONE_YEAR_SECS));
    println!();
    println!("  -> Each trade risks 2% of the running PEAK (cummax), so a");
    println!("     -20 pip loss after a win costs 1.06 * 0.02 = 0.0212.");
}

// ============================================================
// SECTION 3: Unmatched Signals - Warm-Up and Tail Bars Do Not Count
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_unmatched_signals() {
    println!("============================================================");
    println!("SECTION 3: Unmatched Signals - Warm-Up and Tail Bars Do Not Count");
    println!("============================================================");

    // Operation points exist only on days 5..=24: no ATR before, no room to
    // resolve a trade after. Signals, however, fire on any bar.
    let longs: Vec<OperationPoint> = (5..=24).map(|d| op(d, 30, START + i64::from(d) * DAY)).collect();
    let ops = OperationPoints { long_operation_points: &longs, short_operation_points: &[] };
    let day = |d: i64| START + d * DAY;
    let filter = GlobalStrategyFilter::default(); // min 10 ops/yr

    // 12 signals, but only 6 (days 6..=16 even) land on a real trade.
    let edge_inflated: Vec<i64> = [1, 2, 3, 6, 8, 10, 12, 14, 16, 26, 27, 28].map(day).to_vec();
    // 14 signals, 10 of them on days 5..=14.
    let solid: Vec<i64> = [1, 2].into_iter().chain(5..=14).chain([27, 28]).map(day).collect();
    println!("group 1: {} raw signals, group 2: {} raw signals", edge_inflated.len(), solid.len());

    let out = create_strategies(
        &[group(1, edge_inflated, vec![]), group(2, solid, vec![])], &ops, &filter, 390, START, END);
    println!("survivors: {:?}", out.iter().map(|s| s.indicator_id).collect::<Vec<_>>());
    for s in &out {
        println!("  indicator {} annual_ops={} long ids={:?}", s.indicator_id, s.annual_operation_count, s.long_operation_point_ids);
    }
    println!();
    println!("  -> matched_trade_count: group 1 has 6 matched (< 10) -> dropped,");
    println!("     even though its 12 raw signals passed the stage-1 signal filter.");
    println!("  -> Stored annual_ops counts matched trades only (10, not 14).");
}

// ============================================================
// SECTION 4: Long/Short Isolation - Same Timestamp, Separate Maps
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_long_short_isolation() {
    println!("============================================================");
    println!("SECTION 4: Long/Short Isolation - Same Timestamp, Separate Maps");
    println!("============================================================");

    // Upstream generates long and short points in lockstep: every bar has one
    // of each, with DIFFERENT ids and results, keyed by the SAME timestamp.
    let t = START + DAY;
    let longs = vec![op(21, 60, t), op(23, 60, t + DAY), op(25, 60, t + 2 * DAY)];
    let shorts = vec![op(22, -20, t), op(24, -20, t + DAY), op(26, -20, t + 2 * DAY)];
    let ops = OperationPoints { long_operation_points: &longs, short_operation_points: &shorts };

    let cases = [
        ("long only ", group(1, vec![t, t + DAY, t + 2 * DAY], vec![])),
        ("short only", group(2, vec![], vec![t, t + DAY, t + 2 * DAY])),
    ];
    let relaxed = GlobalStrategyFilter { min_annual_operation_count: 0.0, min_annual_roi: -100.0, ..GlobalStrategyFilter::default() };
    for (label, sg) in cases {
        let s = &create_strategies(&[sg], &ops, &relaxed, 390, START, END)[0];
        println!("{label}: long={:?} short={:?} roi={} dd={}",
            s.long_operation_point_ids, s.short_operation_point_ids, s.annual_roi, s.max_draw_down);
    }
    println!();
    println!("  -> A long signal is only ever looked up in long_map, so a short");
    println!("     signal at the same bar can never steal the long trade.");
    println!("  -> One map per side is what makes timestamp a unique key.");
}

// ============================================================
// SECTION 5: Three Gates - Activity, Then Metrics, Then Performance
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_three_gates() {
    println!("============================================================");
    println!("SECTION 5: Three Gates - Activity, Then Metrics, Then Performance");
    println!("============================================================");

    let longs: Vec<OperationPoint> = (1..=12).map(|d| op(d, 60, START + i64::from(d) * DAY)).collect();
    let shorts: Vec<OperationPoint> = (1..=12).map(|d| op(100 + d, -20, START + i64::from(d) * DAY)).collect();
    let ops = OperationPoints { long_operation_points: &longs, short_operation_points: &shorts };
    let days = |n: i64| (1..=n).map(|d| START + d * DAY).collect::<Vec<_>>();
    let filter = GlobalStrategyFilter::default();

    // Reverse indicator order on purpose: output must follow INPUT order.
    let groups = vec![
        group(30, days(12), vec![]), // 12 winning longs -> kept
        group(20, vec![], days(12)), // 12 losing shorts -> dropped at gate 3
        group(10, days(4), vec![]),  // 4 trades/yr -> dropped at gate 1
    ];
    for g in &groups {
        println!("indicator {}: {} signals", g.indicator_id, g.long_signals.len() + g.short_signals.len());
    }
    let out = create_strategies(&groups, &ops, &filter, 390, START, END);
    println!();
    println!("gate 1 admits_activity(matched/yr >= {})  -> drops 10", filter.min_annual_operation_count);
    println!("gate 2 create_strategy                   -> metrics for 30, 20");
    println!("gate 3 admits_performance(roi >= {}, dd <= {}) -> drops 20", filter.min_annual_roi, filter.max_draw_down_max);
    println!();
    println!("survivors (input order kept): {:?}", out.iter().map(|s| (s.indicator_id, s.annual_roi)).collect::<Vec<_>>());
    println!();
    println!("  -> Gate 1 is a cheap hash-lookup count; gate 2 allocates and sorts.");
    println!("     Rejecting early skips the expensive work for most groups.");
    println!("  -> par_iter + collect on an indexed source keeps input order.");
}

// ============================================================
// SECTION 6: Edge Cases - No Groups, Nothing Resolves, Zero Span
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_edge_cases() {
    println!("============================================================");
    println!("SECTION 6: Edge Cases - No Groups, Nothing Resolves, Zero Span");
    println!("============================================================");

    let (longs, shorts) = seed_ops();
    let ops = OperationPoints { long_operation_points: &longs, short_operation_points: &shorts };
    // Accepts everything, so we can see what create_strategy produces.
    let open = GlobalStrategyFilter { max_draw_down_max: 100.0, min_annual_roi: 0.0, min_annual_operation_count: 0.0 };

    let none = create_strategies(&[], &ops, &open, 390, START, END);
    println!("no groups             -> {} strategies", none.len());

    // Signals on bars with no operation point: everything resolves to nothing.
    let ghost = group(5, vec![START + 99 * DAY], vec![START + 98 * DAY]);
    let s = &create_strategies(&[ghost], &ops, &open, 390, START, END)[0];
    println!("nothing resolves      -> roi={} dd={} ops={} ids={:?}/{:?}",
        s.annual_roi, s.max_draw_down, s.annual_operation_count, s.long_operation_point_ids, s.short_operation_point_ids);
    println!("  same with default filter -> {} kept (0 ops/yr < 10)",
        create_strategies(&[group(5, vec![START + 99 * DAY], vec![])], &ops, &GlobalStrategyFilter::default(), 390, START, END).len());

    // start == end: difference_in_years is None, both annualizations return 0.
    let sg = group(6, vec![START + DAY, START + 3 * DAY], vec![START + 2 * DAY]);
    let s = &create_strategies(&[sg], &ops, &open, 390, START, START)[0];
    println!("zero span (start=end) -> roi={} ops={} dd={}", s.annual_roi, s.annual_operation_count, s.max_draw_down);
    println!();
    println!("  -> Zero metrics, not NaN or a panic: the defaults then drop them.");
    println!("  -> max_draw_down is span-independent (it is never annualized).");
}
