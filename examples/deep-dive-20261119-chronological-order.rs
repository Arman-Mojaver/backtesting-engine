//! Deep Dive: validate_chronological_order — resampled points must strictly increase
//! Source: Code input — src/domain/validation/chronological_order_validation.rs
//!
//! The rule module is private: only `validate_all` is public. Sections 1 and 6
//! mirror the function body locally to expose its iterator pipeline; the other
//! sections drive the real rule through `validate_all` with inputs that pass
//! every earlier rule (empty, instrument, MMS, timestamp-match), so any error
//! seen comes from the chronology check itself.

use std::env;

use backtesting_engine::domain::indicator::{Indicator, IndicatorParameters};
use backtesting_engine::domain::operation_point::OperationPoint;
use backtesting_engine::domain::resampled_point::ResampledPointD1;
use backtesting_engine::domain::validation::{ValidationError, validate_all};
use itertools::Itertools;

const MMS: i32 = 390;
// 2026-01-05 .. 2026-01-08 at 00:00 UTC, one D1 resampled point per day.
const MON: i64 = 1_767_571_200;
const TUE: i64 = MON + 86_400;
const WED: i64 = TUE + 86_400;
const THU: i64 = WED + 86_400;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_pipeline(),
        "2" => section_2_accepted(),
        "3" => section_3_rejected(),
        "4" => section_4_duplicate_day(),
        "5" => section_5_error_message(),
        "6" => section_6_why_find_not_all(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-chronological-order <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Pipeline: map -> tuple_windows -> find, step by step");
    println!("  2    - Accepted Inputs: increasing, single point, gaps");
    println!("  3    - Rejected Inputs: inversion and first offending pair");
    println!("  4    - Duplicate Day: why the check is strict (<=, not <)");
    println!("  5    - Error Message: fields and Display text");
    println!("  6    - Why find, Not all: keeping the offending pair");
}

// ---- Seed data ----
fn rp(id: i32, timestamp: i64) -> ResampledPointD1 {
    ResampledPointD1 { id, instrument: "EURUSD".into(), open: 1.0842, high: 1.0871,
        low: 1.0815, close: 1.0857, volume: 48_210, timestamp }
}
fn rps(timestamps: &[i64]) -> Vec<ResampledPointD1> {
    timestamps.iter().zip(1..).map(|(&ts, id)| rp(id, ts)).collect()
}
// One op on MON keeps earlier rules happy: MON is present in every seed list.
fn ops() -> Vec<OperationPoint> {
    vec![OperationPoint { id: 1, instrument: "EURUSD".into(), money_management_strategy_id: MMS,
        timestamp: MON, result: 1, tp: 40, sl: 20, risk: 0.02 }]
}
fn indicators() -> Vec<Indicator> {
    let rsi = IndicatorParameters::Rsi { n: 14, high_threshold: 70, low_threshold: 30 };
    vec![Indicator { id: 4217, parameters: rsi }]
}
fn check(timestamps: &[i64]) -> Result<(), ValidationError> {
    let op = ops();
    validate_all(&indicators(), &rps(timestamps), &op, &op, MMS)
}
fn day(ts: i64) -> &'static str {
    match ts { MON => "MON", TUE => "TUE", WED => "WED", THU => "THU", _ => "???" }
}
fn days(timestamps: &[i64]) -> String {
    timestamps.iter().map(|&ts| day(ts)).join(",")
}
fn show(timestamps: &[i64]) {
    match check(timestamps) {
        Ok(()) => println!("[{:<15}] -> Ok(())", days(timestamps)),
        Err(ValidationError::ResampledPointsNotChronological { previous, timestamp }) => println!(
            "[{:<15}] -> NotChronological {{ previous: {}, timestamp: {} }}",
            days(timestamps), day(previous), day(timestamp)),
        Err(e) => println!("[{:<15}] -> other rule fired: {e:?}", days(timestamps)),
    }
}
fn banner(title: &str) {
    println!("============================================================");
    println!("{title}");
    println!("============================================================");
}

// ============================================================
// SECTION 1: Pipeline - map -> tuple_windows -> find, Step by Step
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_pipeline() {
    banner("SECTION 1: Pipeline - map -> tuple_windows -> find, Step by Step");
    let points = rps(&[MON, WED, TUE, THU]);
    // Step 1: project each point to its timestamp; prices are irrelevant here.
    let timestamps: Vec<i64> = points.iter().map(|p| p.timestamp).collect();
    println!("map(timestamp):  [{}]", days(&timestamps));
    // Step 2: tuple_windows yields overlapping adjacent pairs, n-1 of them.
    println!("tuple_windows():");
    for (previous, timestamp) in timestamps.iter().copied().tuple_windows() {
        let verdict = if timestamp <= previous { "timestamp <= previous -> HIT" } else { "ok" };
        println!("  ({}, {})  {verdict}", day(previous), day(timestamp));
    }
    // Step 3: find stops at the first hit; later pairs are never inspected.
    let hit = timestamps.iter().copied().tuple_windows().find(|(p, t)| t <= p);
    println!("find() -> {:?}", hit.map(|(p, t)| (day(p), day(t))));
    println!();
    println!("Lazy iterators: no Vec of pairs is allocated in the real code,");
    println!("and the walk stops at the first violation.");
}

// ============================================================
// SECTION 2: Accepted Inputs - Increasing, Single Point, Gaps
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_accepted() {
    banner("SECTION 2: Accepted Inputs - Increasing, Single Point, Gaps");
    show(&[MON, TUE, WED, THU]);
    // Gaps (weekends, holidays) are fine: only order is checked, not spacing.
    show(&[MON, THU]);
    // One point -> zero windows -> find returns None -> Ok.
    show(&[MON]);
    println!();
    println!("An empty slice would also yield zero windows and pass this rule,");
    println!("but validate_all rejects it earlier with ResampledPointsEmpty.");
}

// ============================================================
// SECTION 3: Rejected Inputs - Inversion and First Offending Pair
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_rejected() {
    banner("SECTION 3: Rejected Inputs - Inversion and First Offending Pair");
    show(&[MON, WED, TUE]);
    // Two violations (THU>WED, WED>TUE): only the first pair is reported.
    show(&[MON, THU, WED, TUE]);
    // Fully reversed: the very first window already fails.
    show(&[THU, WED, TUE, MON]);
    println!();
    println!("The error names the adjacent pair, not the globally misplaced");
    println!("point: in [MON,WED,TUE] either WED or TUE could be 'wrong'.");
}

// ============================================================
// SECTION 4: Duplicate Day - Why the Check is Strict (<=, not <)
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_duplicate_day() {
    banner("SECTION 4: Duplicate Day - Why the Check is Strict (<=, not <)");
    show(&[MON, TUE, TUE, WED]);
    println!();
    // The earlier timestamp-match rule builds FxHashSets, so a duplicate
    // collapses to one entry and passes. This rule is the only guard.
    println!("Set-based timestamp matching cannot see duplicates; this rule can.");
    println!("Two D1 points for one day would feed indicator math the same day");
    println!("twice, so equality counts as a violation.");
}

// ============================================================
// SECTION 5: Error Message - Fields and Display Text
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_error_message() {
    banner("SECTION 5: Error Message - Fields and Display Text");
    let Err(error) = check(&[MON, WED, TUE]) else {
        println!("unexpected Ok");
        return;
    };
    println!("Debug:   {error:?}");
    println!("Display: {error}");
    println!();
    println!("Raw epoch seconds, previous={WED} (WED), timestamp={TUE} (TUE).");
    println!("Field order mirrors the window: (previous, timestamp).");
}

// ============================================================
// SECTION 6: Why find, Not all - Keeping the Offending Pair
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_why_find_not_all() {
    banner("SECTION 6: Why find, Not all - Keeping the Offending Pair");
    let timestamps = [MON, WED, TUE];
    // A boolean check answers "is it sorted?" but loses which pair broke it.
    let sorted = timestamps.iter().tuple_windows().all(|(p, t)| t > p);
    println!("all(t > p)    -> {sorted}   (no context for the error)");
    let pair = timestamps.iter().copied().tuple_windows().find(|(p, t)| t <= p);
    println!("find(t <= p)  -> {pair:?}");
    println!();
    // `if let Some(..) = .. { return Err(..) }` turns that Option into the error.
    println!("find keeps the pair, which becomes the error's fields.");
    println!("Upstream, pg_adapter's `ORDER BY timestamp` should make this rule");
    println!("never fire; it guards against a query or fixture losing that order.");
}
