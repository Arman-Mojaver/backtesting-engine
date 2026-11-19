//! Deep Dive: run_strategies (strategy runner)
//! Source: Code input — src/application/strategy_runner.rs
//!
//! The per-cell orchestrator: one (instrument, mms) cell in, its Strategy rows out.
//!   validate_all -> time window from LONG ops -> create_signal_groups (cached)
//!   -> stage 1: filter by RAW signal count -> create_strategies
//!   -> stage 2: matched-trade activity + ROI/draw-down
//!
//! Seed data mirrors the module's own tests: 28 daily-ish bars whose RSI(3)
//! cycles 33/67/67/33, and operation points on bars 6..=26 spanning exactly
//! one Julian year, so every annualized figure equals its raw count.

use std::env;

use backtesting_engine::application::signal_generator::{create_signal_groups, signal_group_cache_stats};
use backtesting_engine::application::strategy_generator::create_strategies;
use backtesting_engine::application::strategy_runner::run_strategies;
use backtesting_engine::domain::annual_operation_count::annual_operation_count;
use backtesting_engine::domain::indicator::{Indicator, IndicatorParameters};
use backtesting_engine::domain::operation_point::{OperationPoint, OperationPoints};
use backtesting_engine::domain::resampled_point::ResampledPointD1;
use backtesting_engine::domain::signal_group::filter_signal_groups_by_annual_ops;
use backtesting_engine::domain::strategy::GlobalStrategyFilter;
use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, TermLogger, TerminalMode};

// 20 bars * BAR_SPACING = 365.25 days, so bars 6 -> 26 are exactly one year.
const BAR_SPACING: i64 = 1_577_880;
const MMS: i32 = 1;
const PASSES_BOTH_STAGES: i32 = 7;
const DROPPED_FOR_INACTIVITY: i32 = 8;
const DROPPED_FOR_LOW_ROI: i32 = 9;
const DUPLICATE: i32 = 10;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_happy_path(),
        "2" => section_2_funnel_by_hand(),
        "3" => section_3_time_window(),
        "4" => section_4_validation_gate(),
        "5" => section_5_signal_cache(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-strategy-runner <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Happy Path: four indicators in, two strategies out");
    println!("  2    - The Funnel by Hand: each stage's survivors");
    println!("  3    - Time Window: start/end come from long ops, not bars");
    println!("  4    - Validation Gate: first broken invariant wins");
    println!("  5    - Signal Cache: keyed on (indicator id, instrument)");
}

// ---- Seed data (same shape as strategy_runner's tests) ----

fn bars_with_closes(instrument: &str, closes: [f64; 4]) -> Vec<ResampledPointD1> {
    (0..28)
        .map(|k: i32| {
            let close = closes[(k % 4) as usize];
            ResampledPointD1 {
                id: k + 1,
                instrument: instrument.into(),
                open: close,
                high: close + 0.1,
                low: close - 0.1,
                close,
                volume: 1000,
                timestamp: i64::from(k) * BAR_SPACING,
            }
        })
        .collect()
}

// Closes [5,4,3,4] -> RSI(3) is 33/67/67/33 by bar mod 4: crosses 60/40, never 70/30.
fn bars() -> Vec<ResampledPointD1> {
    bars_with_closes("EURUSD", [5.0, 4.0, 3.0, 4.0])
}

// tp 30 / sl 20 / risk 2% -> each pip moves equity 0.1%; +30 = +3%, -20 = -2%.
fn op(id: i32, result: i32, bar: i32) -> OperationPoint {
    OperationPoint {
        id,
        instrument: "EURUSD".into(),
        money_management_strategy_id: MMS,
        timestamp: i64::from(bar) * BAR_SPACING,
        result,
        tp: 30,
        sl: 20,
        risk: 0.02,
    }
}

// Winners sit exactly where RSI(3) 60/40 enters: longs on bar%4==1, shorts on bar%4==3.
fn ops(winning_bar_mod_4: i32, id_offset: i32) -> Vec<OperationPoint> {
    (6..=26)
        .map(|bar| op(id_offset + bar, if bar % 4 == winning_bar_mod_4 { 30 } else { -20 }, bar))
        .collect()
}

fn longs() -> Vec<OperationPoint> {
    ops(1, 0)
}

fn shorts() -> Vec<OperationPoint> {
    ops(3, 100)
}

fn rsi(id: i32, high_threshold: i32, low_threshold: i32) -> Indicator {
    Indicator { id, parameters: IndicatorParameters::Rsi { n: 3, high_threshold, low_threshold } }
}

fn indicators() -> Vec<Indicator> {
    vec![
        rsi(PASSES_BOTH_STAGES, 60, 40),
        rsi(DROPPED_FOR_INACTIVITY, 70, 30),
        Indicator { id: DROPPED_FOR_LOW_ROI, parameters: IndicatorParameters::Roc { period: 5 } },
        rsi(DUPLICATE, 60, 40),
    ]
}

// ============================================================
// SECTION 1: Happy Path - Four Indicators In, Two Strategies Out
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_happy_path() {
    println!("============================================================");
    println!("SECTION 1: Happy Path - Four Indicators In, Two Strategies Out");
    println!("============================================================");

    // run_strategies reports each filter stage only through debug! — turn it on.
    let config = ConfigBuilder::new().set_time_level(LevelFilter::Off).set_target_level(LevelFilter::Off).build();
    let _ = TermLogger::init(LevelFilter::Debug, config, TerminalMode::Stdout, ColorChoice::Never);

    let ids: Vec<i32> = indicators().iter().map(|i| i.id).collect();
    println!("indicators in: {ids:?}");
    let strategies = match run_strategies(&indicators(), &bars(), &longs(), &shorts(), MMS) {
        Ok(s) => s,
        Err(e) => return println!("unexpected error: {e:#}"),
    };
    println!("strategies out: {}", strategies.len());
    for s in &strategies {
        println!("  indicator {:>2}: roi={} dd={} ops/yr={}", s.indicator_id, s.annual_roi, s.max_draw_down, s.annual_operation_count);
        println!("               long ids={:?}", s.long_operation_point_ids);
        println!("               short ids={:?}", s.short_operation_point_ids);
    }
    println!();
    println!("  -> 10 winners at +3% compounded: 1.03^10 = 1.3439 -> 34.39%.");
    println!("  -> 7 and 10 share parameters and BOTH survive: the runner keys");
    println!("     strategies by indicator id and never dedupes by parameters.");
}

// ============================================================
// SECTION 2: The Funnel by Hand - Each Stage's Survivors
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_funnel_by_hand() {
    println!("============================================================");
    println!("SECTION 2: The Funnel by Hand - Each Stage's Survivors");
    println!("============================================================");

    // Same calls run_strategies makes, minus validation, with each stage printed.
    let (bars, longs, shorts) = (bars(), longs(), shorts());
    let ops = OperationPoints { long_operation_points: &longs, short_operation_points: &shorts };
    let Some((start, end)) = ops.start_end_timestamps() else { return };
    let filter = GlobalStrategyFilter::default();

    let groups = create_signal_groups(&indicators(), &bars);
    println!("create_signal_groups -> {} groups", groups.len());
    for g in &groups {
        let per_year = annual_operation_count(g.long_signals.len() + g.short_signals.len(), start, end);
        println!("  indicator {:>2}: {:>2} long + {:>2} short signals = {per_year}/yr", g.indicator_id, g.long_signals.len(), g.short_signals.len());
    }

    let stage_1 = filter_signal_groups_by_annual_ops(groups, &filter, start, end);
    let ids: Vec<i32> = stage_1.iter().map(|g| g.indicator_id).collect();
    println!("stage 1 (raw signals/yr >= {}) -> {ids:?}", filter.min_annual_operation_count);

    let stage_2 = create_strategies(&stage_1, &ops, &filter, MMS, start, end);
    let kept: Vec<(i32, f64)> = stage_2.iter().map(|s| (s.indicator_id, s.annual_roi)).collect();
    println!("stage 2 (matched ops, roi >= {}, dd <= {}) -> {kept:?}", filter.min_annual_roi, filter.max_draw_down_max);
    println!();
    println!("  -> 8 (RSI 70/30) never fires on a 33/67 series: zero signals, gone at");
    println!("     stage 1 before any metrics are computed.");
    println!("  -> 9 (ROC) is busy enough for stage 1 but its trades lose: only");
    println!("     stage 2 can see that, because only it resolves real results.");
    println!("  -> Stage 1 is the cheap pre-filter: it counts signals, it does not");
    println!("     look them up. Stage 2 recounts on MATCHED trades.");
}

// ============================================================
// SECTION 3: Time Window - Start/End Come From Long Ops, Not Bars
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_time_window() {
    println!("============================================================");
    println!("SECTION 3: Time Window - Start/End Come From Long Ops, Not Bars");
    println!("============================================================");

    let (bars, longs, shorts) = (bars(), longs(), shorts());
    let years = |a: i64, b: i64| (b - a) as f64 / (365.25 * 86_400.0);
    let (first_bar, last_bar) = (bars[0].timestamp, bars[bars.len() - 1].timestamp);
    println!("resampled bars: {first_bar}..{last_bar} ({:.2} yr)", years(first_bar, last_bar));

    let ops = OperationPoints { long_operation_points: &longs, short_operation_points: &shorts };
    let Some((start, end)) = ops.start_end_timestamps() else { return };
    println!("long op window: {start}..{end} ({:.2} yr)  <- what the runner uses", years(start, end));

    // Short ops are ignored on purpose: validation already proved they share
    // the long timestamps, so scanning them would find the same min/max.
    let empty_longs = OperationPoints { long_operation_points: &[], short_operation_points: &shorts };
    println!("no long ops -> start_end_timestamps() = {:?}", empty_longs.start_end_timestamps());
    println!();
    println!("Same 10 trades annualized over each span:");
    println!("  op window   : {}/yr", annual_operation_count(10, start, end));
    println!("  bar window  : {}/yr", annual_operation_count(10, first_bar, last_bar));
    println!();
    println!("  -> Bars include the ATR warm-up and tail with no trades; using them");
    println!("     would dilute every annual figure. The op window is the period a");
    println!("     strategy could actually trade.");
    println!("  -> The None case surfaces as 'long operation points should never");
    println!("     be empty' — unreachable in practice, validate_all rejects it first.");
}

// ============================================================
// SECTION 4: Validation Gate - First Broken Invariant Wins
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_validation_gate() {
    println!("============================================================");
    println!("SECTION 4: Validation Gate - First Broken Invariant Wins");
    println!("============================================================");

    let mut usdcad = shorts();
    usdcad[0].instrument = "USDCAD".into();
    let mut missing_short = shorts();
    missing_short.pop();
    let mut reversed = bars();
    reversed.reverse();

    // (label, indicators, bars, longs, shorts, mms)
    let cases: Vec<(&str, Vec<Indicator>, Vec<ResampledPointD1>, Vec<OperationPoint>, Vec<OperationPoint>, i32)> = vec![
        ("no indicators", vec![], bars(), longs(), shorts(), MMS),
        ("wrong mms id", indicators(), bars(), longs(), shorts(), 2),
        ("one USDCAD short", indicators(), bars(), longs(), usdcad, MMS),
        ("short side missing a bar", indicators(), bars(), longs(), missing_short, MMS),
        ("bars reversed", indicators(), reversed, longs(), shorts(), MMS),
        ("no indicators AND wrong mms", vec![], bars(), longs(), shorts(), 2),
    ];
    for (label, ind, rp, lo, sh, mms) in cases {
        match run_strategies(&ind, &rp, &lo, &sh, mms) {
            Ok(s) => println!("[{label}] ok: {} strategies", s.len()),
            Err(e) => println!("[{label}]\n    {e:#}"),
        }
    }
    println!();
    println!("  -> Every error carries the 'input validation failed' context, then");
    println!("     the ValidationError that names the violated constraint.");
    println!("  -> Checks run in a fixed order (empty, instrument, mms, timestamps,");
    println!("     chronology) and stop at the first: the last case reports only");
    println!("     the empty list. Nothing downstream runs on bad input.");
}

// ============================================================
// SECTION 5: Signal Cache - Keyed on (Indicator Id, Instrument)
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_signal_cache() {
    println!("============================================================");
    println!("SECTION 5: Signal Cache - Keyed on (Indicator Id, Instrument)");
    println!("============================================================");

    let run = |label: &str, bars: &[ResampledPointD1]| {
        let strategies = run_strategies(&[rsi(PASSES_BOTH_STAGES, 60, 40)], bars, &longs(), &shorts(), MMS);
        let count = strategies.map_or(0, |s| s.len());
        let stats = signal_group_cache_stats();
        println!("{label:<34} -> {count} strategy | hits={} misses={}", stats.hits, stats.misses);
    };

    run("1. EURUSD, RSI-friendly closes", &bars());
    run("2. same call again", &bars());
    // Flat closes: RSI would never cross anything if it were recomputed.
    run("3. EURUSD, FLAT closes", &bars_with_closes("EURUSD", [4.0; 4]));
    println!();
    println!("  -> Run 3 still returns a strategy: the key is (id, instrument), not");
    println!("     the bars, so it reuses the signals computed for run 1.");
    println!("  -> Safe in production because an instrument's resampled points are");
    println!("     loaded once and never change within a process. Different data");
    println!("     for the same instrument in one process WILL read stale signals.");
    println!("  -> The memo is process-wide (not per run_strategies call), which is");
    println!("     why every mms cell of an instrument after the first is a hit.");
}
