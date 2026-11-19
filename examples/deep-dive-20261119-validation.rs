//! Deep Dive: domain::validation — the input gate before a backtest runs
//! Source: Code input — src/domain/validation/mod.rs (+ its five rule submodules)
//!
//! `run_strategies` calls `validate_all` before any signal is generated. Only
//! `validate_all` and `ValidationError` are public; the five rule modules are
//! private, so every section drives a rule through `validate_all` with inputs
//! that pass all earlier rules and trip exactly the one under study.
//! Timestamps use 1000-steps: the rules only compare equality and order.

use std::env;

use anyhow::Context;
use backtesting_engine::domain::indicator::{Indicator, IndicatorParameters};
use backtesting_engine::domain::operation_point::OperationPoint;
use backtesting_engine::domain::resampled_point::ResampledPointD1;
use backtesting_engine::domain::validation::{ValidationError, validate_all};

const MMS: i32 = 390;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_error_catalog(),
        "2" => section_2_happy_path(),
        "3" => section_3_empty_precedence(),
        "4" => section_4_instrument(),
        "5" => section_5_mms(),
        "6" => section_6_timestamps(),
        "7" => section_7_chronology(),
        "8" => section_8_fail_fast(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-validation <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Error Catalog: every ValidationError and its message");
    println!("  2    - Happy Path: consistent inputs, resampled wider than ops");
    println!("  3    - Empty Precedence: which empty list is reported first");
    println!("  4    - Instrument Check: what 'expected' actually means");
    println!("  5    - MMS Check: caller-supplied id vs every operation point");
    println!("  6    - Timestamp Check: set differences, smallest offender wins");
    println!("  7    - Chronology Check: strictly increasing, first bad pair");
    println!("  8    - Fail Fast: one fault reported, wrapped by strategy_runner");
}

// ---- Seed data ----
fn rp_on(instrument: &str, timestamp: i64) -> ResampledPointD1 {
    let id = i32::try_from(timestamp / 1000).unwrap_or_default();
    ResampledPointD1 { id, instrument: instrument.into(), open: 1.0842, high: 1.0871,
        low: 1.0815, close: 1.0857, volume: 48_210, timestamp }
}
fn rps(timestamps: &[i64]) -> Vec<ResampledPointD1> {
    timestamps.iter().map(|&ts| rp_on("EURUSD", ts)).collect()
}
fn op_full(instrument: &str, mms: i32, timestamp: i64) -> OperationPoint {
    let id = i32::try_from(timestamp / 1000).unwrap_or_default();
    OperationPoint { id, instrument: instrument.into(), money_management_strategy_id: mms,
        timestamp, result: 1, tp: 40, sl: 20, risk: 0.02 }
}
fn ops(timestamps: &[i64]) -> Vec<OperationPoint> {
    timestamps.iter().map(|&ts| op_full("EURUSD", MMS, ts)).collect()
}
fn indicators() -> Vec<Indicator> {
    let rsi = IndicatorParameters::Rsi { n: 14, high_threshold: 70, low_threshold: 30 };
    vec![Indicator { id: 4217, parameters: rsi }]
}

fn show(label: &str, result: &Result<(), ValidationError>) {
    match result {
        Ok(()) => println!("{label:<34} -> Ok(())"),
        Err(e) => println!("{label:<34} -> {e:?}"),
    }
}

fn banner(title: &str) {
    println!("============================================================");
    println!("{title}");
    println!("============================================================");
}

// ============================================================
// SECTION 1: Error Catalog - Every ValidationError and Its Message
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_error_catalog() {
    banner("SECTION 1: Error Catalog - Every ValidationError and Its Message");
    // Variant order in the enum mirrors the order validate_all checks them.
    let all = [
        ValidationError::LongOperationPointsEmpty,
        ValidationError::ShortOperationPointsEmpty,
        ValidationError::IndicatorsEmpty,
        ValidationError::ResampledPointsEmpty,
        ValidationError::InstrumentMismatch { expected: "EURUSD".into(), found: "USDCAD".into() },
        ValidationError::MoneyManagementStrategyMismatch { expected: MMS, found: 391 },
        ValidationError::LongShortTimestampsDiffer { timestamp: 4000 },
        ValidationError::TimestampMissingFromResampled { timestamp: 4000 },
        ValidationError::ResampledPointsNotChronological { previous: 3000, timestamp: 2000 },
    ];
    for (n, e) in all.iter().enumerate() {
        println!("{}. {e}", n + 1);
    }
    println!();
    println!("Derives Clone + PartialEq + Eq so tests can assert_eq! on errors;");
    println!("thiserror supplies Display and std::error::Error (no source()).");
}

// ============================================================
// SECTION 2: Happy Path - Consistent Inputs, Resampled Wider Than Ops
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_happy_path() {
    banner("SECTION 2: Happy Path - Consistent Inputs, Resampled Wider Than Ops");
    let resampled = rps(&[1000, 2000, 3000, 4000, 5000]);
    let long = ops(&[2000, 3000, 4000]);
    let short = ops(&[2000, 3000, 4000]);
    println!("resampled ts: 1000..=5000 (5 points)   long/short ts: 2000..=4000");
    show("validate_all", &validate_all(&indicators(), &resampled, &long, &short, MMS));
    println!();
    println!("Ops only need to be a SUBSET of resampled timestamps: the edges");
    println!("(ATR warm-up at the start, unlabelled tail bars) have no ops.");
    show("ops over all 5 days", &validate_all(&indicators(), &resampled,
        &ops(&[1000, 2000, 3000, 4000, 5000]), &ops(&[1000, 2000, 3000, 4000, 5000]), MMS));
}

// ============================================================
// SECTION 3: Empty Precedence - Which Empty List Is Reported First
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_empty_precedence() {
    banner("SECTION 3: Empty Precedence - Which Empty List Is Reported First");
    let (ind, op) = (indicators(), ops(&[1000]));
    // Order: long -> short -> indicators -> resampled, independent of arg order.
    show("everything empty", &validate_all(&[], &[], &[], &[], MMS));
    show("only long present", &validate_all(&[], &[], &op, &[], MMS));
    show("long+short present", &validate_all(&[], &[], &op, &op, MMS));
    show("only resampled missing", &validate_all(&ind, &[], &op, &op, MMS));
    println!();
    println!("Running this first matters: later rules (e.g. timestamps) would");
    println!("trivially pass on empty slices and hide the real problem.");
}

// ============================================================
// SECTION 4: Instrument Check - What 'Expected' Actually Means
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_instrument() {
    banner("SECTION 4: Instrument Check - What 'Expected' Actually Means");
    let ind = indicators();
    let usdcad_rp = vec![rp_on("USDCAD", 1000)];
    let mixed_rp = vec![rp_on("EURUSD", 1000), rp_on("GBPUSD", 2000)];
    let usdcad_short = vec![op_full("USDCAD", MMS, 1000)];
    let (rp, op) = (rps(&[1000]), ops(&[1000]));
    // One chain: resampled, then long, then short; all_equal_value() yields
    // the first value and the first value that differs from it.
    show("resampled USDCAD, ops EURUSD", &validate_all(&ind, &usdcad_rp, &op, &op, MMS));
    show("short USDCAD, rest EURUSD", &validate_all(&ind, &rp, &op, &usdcad_short, MMS));
    show("resampled EURUSD then GBPUSD", &validate_all(&ind, &mixed_rp, &op, &op, MMS));
    println!();
    println!("'expected' = whatever instrument appears first in the chain,");
    println!("not a source of truth. Indicators carry no instrument.");
}

// ============================================================
// SECTION 5: MMS Check - Caller-Supplied Id vs Every Operation Point
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_mms() {
    banner("SECTION 5: MMS Check - Caller-Supplied Id vs Every Operation Point");
    let (ind, rp, op) = (indicators(), rps(&[1000, 2000]), ops(&[1000, 2000]));
    let short_partial = vec![op_full("EURUSD", MMS, 1000), op_full("EURUSD", 391, 2000)];
    // Here 'expected' IS authoritative: it is the id the caller asked for.
    show("caller asks for 390", &validate_all(&ind, &rp, &op, &op, MMS));
    show("caller asks for 391", &validate_all(&ind, &rp, &op, &op, 391));
    show("one short op on 391", &validate_all(&ind, &rp, &op, &short_partial, MMS));
    println!();
    println!("Scans long then short and stops at the first mismatched id.");
}

// ============================================================
// SECTION 6: Timestamp Check - Set Differences, Smallest Offender Wins
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_timestamps() {
    banner("SECTION 6: Timestamp Check - Set Differences, Smallest Offender Wins");
    let ind = indicators();
    let rp = rps(&[1000, 2000, 3000]);
    // Step 1: long XOR short (symmetric difference); report its min.
    show("long {1,2,3}k short {1,2,4}k", &validate_all(&ind, &rp, &ops(&[1000, 2000, 3000]), &ops(&[1000, 2000, 4000]), MMS));
    show("long {1,2}k  short {1,2,4}k", &validate_all(&ind, &rp, &ops(&[1000, 2000]), &ops(&[1000, 2000, 4000]), MMS));
    // Step 2: long minus resampled; short == long by now, so it's not re-checked.
    show("long=short {2,4,5}k", &validate_all(&ind, &rp, &ops(&[2000, 4000, 5000]), &ops(&[2000, 4000, 5000]), MMS));
    println!();
    println!("Hash sets drop multiplicity and order. These therefore pass here:");
    show("long [1k,1k] short [1k]", &validate_all(&ind, &rp, &ops(&[1000, 1000]), &ops(&[1000]), MMS));
    show("long [3k,1k] short [1k,3k]", &validate_all(&ind, &rp, &ops(&[3000, 1000]), &ops(&[1000, 3000]), MMS));
    println!("min() makes the report deterministic despite FxHashSet iteration order.");
}

// ============================================================
// SECTION 7: Chronology Check - Strictly Increasing, First Bad Pair
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_7_chronology() {
    banner("SECTION 7: Chronology Check - Strictly Increasing, First Bad Pair");
    let ind = indicators();
    let op = ops(&[1000]);
    // tuple_windows() walks adjacent (previous, timestamp) pairs; `<=` fails.
    show("resampled [1k,2k,3k]", &validate_all(&ind, &rps(&[1000, 2000, 3000]), &op, &op, MMS));
    show("resampled [1k,3k,2k]", &validate_all(&ind, &rps(&[1000, 3000, 2000]), &op, &op, MMS));
    show("resampled [1k,2k,2k] (dup)", &validate_all(&ind, &rps(&[1000, 2000, 2000]), &op, &op, MMS));
    show("resampled [1k,4k,3k,2k]", &validate_all(&ind, &rps(&[1000, 4000, 3000, 2000]), &op, &op, MMS));
    println!();
    println!("Only resampled points are order-checked: indicator computation");
    println!("walks them sequentially. A duplicate day slips through the set-");
    println!("based timestamp rule (section 6) and is caught here instead.");
}

// ============================================================
// SECTION 8: Fail Fast - One Fault Reported, Wrapped by strategy_runner
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_8_fail_fast() {
    banner("SECTION 8: Fail Fast - One Fault Reported, Wrapped by strategy_runner");
    // Three faults at once: wrong instrument, wrong MMS id, out-of-order points.
    let bad_rp = vec![rp_on("USDCAD", 2000), rp_on("USDCAD", 1000)];
    let bad_op = vec![op_full("EURUSD", 391, 1000)];
    let result = validate_all(&indicators(), &bad_rp, &bad_op, &bad_op, MMS);
    show("3 faults present", &result);
    println!("Each rule ends in `?`, so only the earliest rule's error surfaces.");
    println!();
    // Mirrors application::strategy_runner::run_strategies.
    let wrapped = result.context("input validation failed");
    if let Err(e) = wrapped {
        println!("strategy_runner sees ({{:#}}):");
        println!("  {e:#}");
    }
}
