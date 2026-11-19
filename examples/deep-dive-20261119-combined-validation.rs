//! Deep Dive: combined_validation::validate_all — the orchestration, not the rules
//! Source: Code input — src/domain/validation/combined_validation.rs
//!
//! `validate_all` owns no rule of its own: it chains five private validators
//! with `?`. What it does own is the ORDER, and that order decides which
//! error a caller sees, which faults mask others, and what slips through.
//! (Per-rule details live in deep-dive-20261119-validation.)

use std::env;

use backtesting_engine::domain::indicator::{Indicator, IndicatorParameters};
use backtesting_engine::domain::operation_point::OperationPoint;
use backtesting_engine::domain::resampled_point::ResampledPointD1;
use backtesting_engine::domain::validation::{ValidationError, validate_all};

const MMS: i32 = 390;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_pipeline_map(),
        "2" => section_2_precedence_ladder(),
        "3" => section_3_empty_first(),
        "4" => section_4_root_cause_order(),
        "5" => section_5_coverage_gaps(),
        "6" => section_6_swapped_arguments(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-combined-validation <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Pipeline Map: five stages, what each one reads");
    println!("  2    - Precedence Ladder: fix one fault at a time");
    println!("  3    - Empty First: why emptiness is checked before anything");
    println!("  4    - Root Cause Order: instrument before timestamps");
    println!("  5    - Coverage Gaps: inputs validate_all lets through");
    println!("  6    - Swapped Arguments: two slices of the same type");
}

// ---- Seed data ----
fn rp_on(instrument: &str, timestamp: i64) -> ResampledPointD1 {
    let id = i32::try_from(timestamp / 1000).unwrap_or_default();
    ResampledPointD1 { id, instrument: instrument.into(), open: 1.0842, high: 1.0871,
        low: 1.0815, close: 1.0857, volume: 48_210, timestamp }
}
fn rps_on(instrument: &str, timestamps: &[i64]) -> Vec<ResampledPointD1> {
    timestamps.iter().map(|&ts| rp_on(instrument, ts)).collect()
}
fn op_full(instrument: &str, mms: i32, timestamp: i64) -> OperationPoint {
    let id = i32::try_from(timestamp / 1000).unwrap_or_default();
    OperationPoint { id, instrument: instrument.into(), money_management_strategy_id: mms,
        timestamp, result: 1, tp: 40, sl: 20, risk: 0.02 }
}
fn ops(timestamps: &[i64]) -> Vec<OperationPoint> {
    timestamps.iter().map(|&ts| op_full("EURUSD", MMS, ts)).collect()
}
fn rsi(id: i32) -> Indicator {
    Indicator { id, parameters: IndicatorParameters::Rsi { n: 14, high_threshold: 70, low_threshold: 30 } }
}

fn show(label: &str, result: &Result<(), ValidationError>) {
    match result {
        Ok(()) => println!("{label:<30} -> Ok(())"),
        Err(e) => println!("{label:<30} -> {e:?}"),
    }
}

fn banner(title: &str) {
    println!("============================================================");
    println!("{title}");
    println!("============================================================");
}

// ============================================================
// SECTION 1: Pipeline Map - Five Stages, What Each One Reads
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_pipeline_map() {
    banner("SECTION 1: Pipeline Map - Five Stages, What Each One Reads");
    println!("stage  validator                   indicators resampled long short mms");
    println!("1      empty_entities              x          x         x    x");
    println!("2      same_instrument                        x         x    x");
    println!("3      mms_id_matches                                   x    x     x");
    println!("4      timestamps_match                       x         x    x");
    println!("5      chronological_order                    x");
    println!();
    println!("Indicators are read by stage 1 only; expected_mms_id by stage 3 only.");
    println!("Each call ends in `?`: the first Err returns, later stages never run.");
    println!();
    let (rp, op) = (rps_on("EURUSD", &[1000, 2000, 3000]), ops(&[2000, 3000]));
    show("all five stages pass", &validate_all(&[rsi(1)], &rp, &op, &op, MMS));
}

// ============================================================
// SECTION 2: Precedence Ladder - Fix One Fault at a Time
// (Copy the title above and search this file to find this code!)
// ============================================================
type Inputs = (Vec<Indicator>, Vec<ResampledPointD1>, Vec<OperationPoint>, Vec<OperationPoint>);

// Five planted faults, one per stage; `fixed` = how many are repaired.
fn ladder_inputs(fixed: usize) -> Inputs {
    let indicators = if fixed > 0 { vec![rsi(1)] } else { vec![] };
    let instrument = if fixed > 1 { "EURUSD" } else { "USDCAD" };
    let short_mms = if fixed > 2 { MMS } else { 391 };
    let short_ts: &[i64] = if fixed > 3 { &[1000, 2000, 3000] } else { &[1000, 2000] };
    let rp_ts: &[i64] = if fixed > 4 { &[1000, 2000, 3000] } else { &[1000, 3000, 2000] };
    let short = short_ts.iter().map(|&ts| op_full("EURUSD", short_mms, ts)).collect();
    (indicators, rps_on(instrument, rp_ts), ops(&[1000, 2000, 3000]), short)
}

fn section_2_precedence_ladder() {
    banner("SECTION 2: Precedence Ladder - Fix One Fault at a Time");
    let steps = ["all 5 faults", "+ indicators added", "+ resampled on EURUSD",
        "+ short ops on MMS 390", "+ short gets ts 3000", "+ resampled sorted"];
    for (fixed, label) in steps.iter().enumerate() {
        let (ind, rp, long, short) = ladder_inputs(fixed);
        show(label, &validate_all(&ind, &rp, &long, &short, MMS));
    }
    println!();
    println!("A caller fixing errors one report at a time needs up to five runs:");
    println!("the result is a single ValidationError, never a list.");
}

// ============================================================
// SECTION 3: Empty First - Why Emptiness Is Checked Before Anything
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_empty_first() {
    banner("SECTION 3: Empty First - Why Emptiness Is Checked Before Anything");
    let (rp, op) = (rps_on("EURUSD", &[1000]), ops(&[1000]));
    show("indicators empty", &validate_all(&[], &rp, &op, &op, MMS));
    show("resampled empty", &validate_all(&[rsi(1)], &[], &op, &op, MMS));
    println!();
    println!("Without stage 1:");
    println!("  indicators empty -> nothing else reads them -> Ok(()),");
    println!("    and the runner would produce zero strategies silently.");
    println!("  resampled empty  -> stage 4 reports TimestampMissingFromResampled");
    println!("    {{ timestamp: 1000 }}: true, but it hides the real cause.");
    println!("  all ops empty    -> stages 2-5 iterate nothing and all pass.");
}

// ============================================================
// SECTION 4: Root Cause Order - Instrument Before Timestamps
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_root_cause_order() {
    banner("SECTION 4: Root Cause Order - Instrument Before Timestamps");
    // Wrong file loaded: USDCAD days that don't line up with EURUSD ops.
    let usdcad = rps_on("USDCAD", &[5000, 6000, 7000]);
    let op = ops(&[1000, 2000]);
    show("USDCAD rp, EURUSD ops", &validate_all(&[rsi(1)], &usdcad, &op, &op, MMS));
    println!();
    println!("Both stage 2 and stage 4 would fail on this input. Running the");
    println!("instrument check first names the cause (wrong instrument) rather");
    println!("than a symptom (TimestampMissingFromResampled {{ timestamp: 1000 }}).");
    println!();
    println!("A fault that only stage 5 sees (unsorted but correct days) is");
    println!("reported only after stages 1-4 are clean - see section 2.");
}

// ============================================================
// SECTION 5: Coverage Gaps - Inputs validate_all Lets Through
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_coverage_gaps() {
    banner("SECTION 5: Coverage Gaps - Inputs validate_all Lets Through");
    let rp = rps_on("EURUSD", &[1000, 2000, 3000]);
    let ok = ops(&[1000, 2000, 3000]);
    // Stage 5 only orders resampled points; ops are compared as sets.
    show("ops reversed [3k,2k,1k]", &validate_all(&[rsi(1)], &rp, &ops(&[3000, 2000, 1000]), &ok, MMS));
    show("long has 1000 twice", &validate_all(&[rsi(1)], &rp, &ops(&[1000, 1000, 2000, 3000]), &ok, MMS));
    // Stage 1 checks len() only: duplicate indicator ids are not inspected.
    show("same indicator twice", &validate_all(&[rsi(1), rsi(1)], &rp, &ok, &ok, MMS));
    // Stage 4 checks long ⊆ resampled; short == long already, so implied.
    show("short ⊆ resampled (implied)", &validate_all(&[rsi(1)], &rp, &ok, &ok, MMS));
    println!();
    println!("Downstream, points_by_timestamp collects ops into an FxHashMap:");
    println!("order is irrelevant, but a duplicated timestamp keeps the LAST op.");
}

// ============================================================
// SECTION 6: Swapped Arguments - Two Slices of the Same Type
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_swapped_arguments() {
    banner("SECTION 6: Swapped Arguments - Two Slices of the Same Type");
    let rp = rps_on("EURUSD", &[1000]);
    let (long, short): (Vec<OperationPoint>, Vec<OperationPoint>) = (vec![], ops(&[1000]));
    show("(long, short)", &validate_all(&[rsi(1)], &rp, &long, &short, MMS));
    show("(short, long) swapped", &validate_all(&[rsi(1)], &rp, &short, &long, MMS));
    println!();
    println!("long and short are both &[OperationPoint]: the compiler cannot");
    println!("catch a swap. Validation still fails, but names the wrong side.");
    println!("Once both are non-empty a swap is invisible: every later stage");
    println!("treats long and short symmetrically.");
    println!();
    println!("expected_mms_id is a bare i32 last argument; the only caller,");
    println!("application::strategy_runner::run_strategies, passes its own");
    println!("money_management_strategy_id and wraps errors with");
    println!("  .context(\"input validation failed\")");
}
