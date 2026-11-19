//! Deep Dive: strategy_batch (the `create_loop` batch driver)
//! Source: Code input — src/application/strategy_batch.rs
//!
//! One run of `bte strategy create_loop`:
//!   SourceIndex::load (4 DB queries in parallel) -> cells() (union of long/short keys)
//!   -> compute_cells (par_iter over cells, compute_cell each) -> split_outcomes
//!   -> BatchSummary -> fail_on_total_failure
//!
//! SourceIndex's fields, compute_cell, CellOutcome and split_outcomes are private and
//! load() needs Postgres, so `MirrorIndex` / `mirror_compute_cell` below copy those few
//! lines verbatim. Everything they call (run_strategies, unprocessed_indicators,
//! indicator_ids, BatchSummary, fail_on_total_failure) is the real crate code.
//!
//! Seed data is the strategy-runner deep dive's: RSI(3) cycling 33/67 over 28 bars and
//! operation points on bars 6..=26 spanning one Julian year, so RSI 60/40 yields a strategy.

use std::env;

use backtesting_engine::application::cell_output::CellOutput;
use backtesting_engine::application::strategy_batch::{BatchSummary, fail_on_total_failure};
use backtesting_engine::application::strategy_runner::run_strategies;
use backtesting_engine::domain::cell::Cell;
use backtesting_engine::domain::indicator::{Indicator, IndicatorParameters, indicator_ids, unprocessed_indicators};
use backtesting_engine::domain::operation_point::OperationPoint;
use backtesting_engine::domain::resampled_point::ResampledPointD1;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

const BAR_SPACING: i64 = 1_577_880; // 20 bars = 365.25 days

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_pipeline(),
        "2" => section_2_cells(),
        "3" => section_3_three_outcomes(),
        "4" => section_4_incremental(),
        "5" => section_5_total_failure(),
        "6" => section_6_order(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-strategy-batch <section>");
    println!();
    println!("  CLI DETECTED (create_loop) - section 1 prints commands, never runs them");
    println!();
    println!("  list - Show this list");
    println!("  1    - The Pipeline: what create_loop does end to end");
    println!("  2    - cells(): sorted union of long and short keys");
    println!("  3    - Three Outcomes: computed, already processed, failed");
    println!("  4    - Incremental Runs: only unprocessed indicators are computed");
    println!("  5    - Total Failure: when a partial run becomes an error");
    println!("  6    - Parallel but Ordered: par_iter keeps cell order");
}

// ---- Seed data ----

fn bars(instrument: &str) -> Vec<ResampledPointD1> {
    let closes = [5.0, 4.0, 3.0, 4.0];
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

// Winners sit where RSI(3) 60/40 enters: longs on bar%4==1, shorts on bar%4==3.
fn ops(instrument: &str, mms: i32, winning_bar_mod_4: i32, id_offset: i32) -> Vec<OperationPoint> {
    (6..=26)
        .map(|bar| OperationPoint {
            id: id_offset + bar,
            instrument: instrument.into(),
            money_management_strategy_id: mms,
            timestamp: i64::from(bar) * BAR_SPACING,
            result: if bar % 4 == winning_bar_mod_4 { 30 } else { -20 },
            tp: 30,
            sl: 20,
            risk: 0.02,
        })
        .collect()
}

fn rsi(id: i32) -> Indicator {
    Indicator { id, parameters: IndicatorParameters::Rsi { n: 3, high_threshold: 60, low_threshold: 40 } }
}

// ---- Mirror of the private parts of strategy_batch.rs ----

struct MirrorIndex {
    indicators: Vec<Indicator>,
    resampled_by_instrument: FxHashMap<String, Vec<ResampledPointD1>>,
    long_by_cell: FxHashMap<Cell, Vec<OperationPoint>>,
    short_by_cell: FxHashMap<Cell, Vec<OperationPoint>>,
}

impl MirrorIndex {
    // Same body as SourceIndex::cells.
    fn cells(&self) -> Vec<Cell> {
        let mut cells: Vec<Cell> = self.long_by_cell.keys().chain(self.short_by_cell.keys()).cloned().collect();
        cells.sort_unstable();
        cells.dedup();
        cells
    }
}

// Seeds EURUSD/1 and EURUSD/2 with data, GBPUSD/1 with ops but NO resampled points.
fn mirror_index(indicators: Vec<Indicator>) -> MirrorIndex {
    let mut long_by_cell = FxHashMap::default();
    let mut short_by_cell = FxHashMap::default();
    for (instrument, mms) in [("EURUSD", 1), ("EURUSD", 2), ("GBPUSD", 1)] {
        long_by_cell.insert(Cell::new(instrument, mms), ops(instrument, mms, 1, 0));
        short_by_cell.insert(Cell::new(instrument, mms), ops(instrument, mms, 3, 100));
    }
    MirrorIndex {
        indicators,
        resampled_by_instrument: FxHashMap::from_iter([("EURUSD".to_string(), bars("EURUSD"))]),
        long_by_cell,
        short_by_cell,
    }
}

#[derive(Debug)]
enum MirrorOutcome {
    Computed(CellOutput),
    AlreadyProcessed,
    Failed(Cell),
}

// Same logic as compute_cell; the only change is println! instead of error!.
fn mirror_compute_cell(source: &MirrorIndex, cell: &Cell, processed: &FxHashMap<Cell, FxHashSet<i32>>) -> MirrorOutcome {
    let empty = FxHashSet::default();
    let processed_ids = processed.get(cell).unwrap_or(&empty);
    let unprocessed = unprocessed_indicators(&source.indicators, processed_ids);
    if unprocessed.is_empty() {
        return MirrorOutcome::AlreadyProcessed;
    }
    let resampled = source.resampled_by_instrument.get(&cell.instrument).map_or(&[][..], Vec::as_slice);
    let long = source.long_by_cell.get(cell).map_or(&[][..], Vec::as_slice);
    let short = source.short_by_cell.get(cell).map_or(&[][..], Vec::as_slice);
    match run_strategies(&unprocessed, resampled, long, short, cell.mms_id) {
        Ok(strategies) => MirrorOutcome::Computed(CellOutput {
            cell: cell.clone(),
            strategies,
            computed_indicator_ids: indicator_ids(&unprocessed),
        }),
        Err(e) => {
            println!("  error! -> {cell}: {e:#}");
            MirrorOutcome::Failed(cell.clone())
        }
    }
}

// Same logic as split_outcomes, building the real (pub-field) BatchSummary.
fn mirror_split(outcomes: Vec<MirrorOutcome>) -> (Vec<CellOutput>, BatchSummary) {
    let (mut computed, mut skipped, mut failed_cells) = (Vec::new(), 0usize, Vec::new());
    for outcome in outcomes {
        match outcome {
            MirrorOutcome::Computed(output) => computed.push(output),
            MirrorOutcome::AlreadyProcessed => skipped = skipped.saturating_add(1),
            MirrorOutcome::Failed(cell) => failed_cells.push(cell),
        }
    }
    let summary = BatchSummary { computed: computed.len(), skipped, failed_cells };
    (computed, summary)
}

// ============================================================
// SECTION 1: The Pipeline - What create_loop Does End to End
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_pipeline() {
    println!("============================================================");
    println!("SECTION 1: The Pipeline - What create_loop Does End to End");
    println!("============================================================");
    // Touches Postgres and rewrites parquet partitions: printed, never executed.
    println!("Run yourself (needs the DB up):  ENVIRONMENT=development cargo run --bin bte -- strategy create_loop");
    println!();
    println!("src/engine/strategy/create_loop.rs::execute");
    println!("  1. SourceIndex::load  - 4 PgStore connections, nested rayon::join:");
    println!("       indicators | resampled by instrument | long by cell | short by cell");
    println!("     all 4 queries run concurrently; each error gets its own context");
    println!("  2. source.cells()     - every (instrument, mms) with operation points");
    println!("  3. read_processed_ids_per_cell - manifest: indicator ids done per cell");
    println!("  4. compute_cells      - par_iter: compute_cell per cell -> CellOutcome");
    println!("  5. persist_cells      - write only the Computed outputs");
    println!("  6. report_run         - log counts, list failed cells for retry");
    println!("  7. fail_on_total_failure - exit non-zero only on 'no progress'");
    println!();
    println!("  -> Everything is loaded ONCE up front; the per-cell work is pure and");
    println!("     DB-free, which is what makes the par_iter in step 4 safe.");
    println!("  -> Steps 3 and 5 make the loop resumable: a rerun skips finished work.");
}

// ============================================================
// SECTION 2: cells() - Sorted Union of Long and Short Keys
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_cells() {
    println!("============================================================");
    println!("SECTION 2: cells() - Sorted Union of Long and Short Keys");
    println!("============================================================");
    let v = Vec::new;
    let source = MirrorIndex {
        indicators: Vec::new(),
        resampled_by_instrument: FxHashMap::default(),
        long_by_cell: FxHashMap::from_iter([(Cell::new("USDJPY", 1), v()), (Cell::new("EURUSD", 2), v())]),
        short_by_cell: FxHashMap::from_iter([(Cell::new("EURUSD", 2), v()), (Cell::new("EURUSD", 1), v())]),
    };
    let show = |m: &FxHashMap<Cell, Vec<OperationPoint>>| m.keys().map(ToString::to_string).collect::<Vec<_>>();
    println!("long keys : {:?}", show(&source.long_by_cell));
    println!("short keys: {:?}", show(&source.short_by_cell));
    println!("cells()   : {:?}", source.cells().iter().map(ToString::to_string).collect::<Vec<_>>());
    println!();
    println!("  -> EURUSD/2 is in both maps but appears once (sort + dedup).");
    println!("  -> EURUSD/1 has only shorts and USDJPY/1 only longs, yet both are");
    println!("     cells: a one-sided cell still runs, and run_strategies'");
    println!("     validation reports the missing side as a Failed cell.");
    println!("  -> Cell derives Ord as (instrument, mms_id), so the order is");
    println!("     deterministic regardless of FxHashMap iteration order.");
    println!("  -> Resampled points are keyed by INSTRUMENT, not cell: every mms");
    println!("     cell of EURUSD borrows the same bar slice (no clones).");
}

// ============================================================
// SECTION 3: Three Outcomes - Computed, Already Processed, Failed
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_three_outcomes() {
    println!("============================================================");
    println!("SECTION 3: Three Outcomes - Computed, Already Processed, Failed");
    println!("============================================================");
    let source = mirror_index(vec![rsi(7)]);
    // EURUSD/2 already has indicator 7 in the manifest.
    let processed = FxHashMap::from_iter([(Cell::new("EURUSD", 2), FxHashSet::from_iter([7]))]);
    for cell in source.cells() {
        println!("{cell}:");
        match mirror_compute_cell(&source, &cell, &processed) {
            MirrorOutcome::Computed(o) => println!("  Computed: {} strategies, ids {:?}", o.strategies.len(), o.computed_indicator_ids),
            other => println!("  {other:?}"),
        }
    }
    println!();
    println!("  -> EURUSD/1: absent from the manifest -> empty set -> all unprocessed.");
    println!("  -> EURUSD/2: nothing left to do, returns BEFORE run_strategies runs.");
    println!("  -> GBPUSD/1: has ops but no bars; resampled_points() falls back to an");
    println!("     empty slice instead of panicking, so validation rejects it and");
    println!("     the error is logged per cell. One bad cell never aborts the batch.");
}

// ============================================================
// SECTION 4: Incremental Runs - Only Unprocessed Indicators Are Computed
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_incremental() {
    println!("============================================================");
    println!("SECTION 4: Incremental Runs - Only Unprocessed Indicators Are Computed");
    println!("============================================================");
    // 7 and 8 have identical parameters; only their ids differ.
    let source = mirror_index(vec![rsi(7), rsi(8)]);
    let cell = Cell::new("EURUSD", 1);
    for done in [vec![], vec![7], vec![7, 8], vec![99]] {
        let processed = FxHashMap::from_iter([(cell.clone(), done.iter().copied().collect())]);
        let label = format!("manifest {done:?}");
        match mirror_compute_cell(&source, &cell, &processed) {
            MirrorOutcome::Computed(o) => println!("{label:<18} -> computes {:?} ({} strategies)", o.computed_indicator_ids, o.strategies.len()),
            other => println!("{label:<18} -> {other:?}"),
        }
    }
    println!();
    println!("  -> computed_indicator_ids is every unprocessed indicator that was RUN,");
    println!("     not only those that produced a strategy. Persisting it marks the");
    println!("     filtered-out ones as done too, so they are not retried forever.");
    println!("  -> An unknown id in the manifest (99) is simply ignored.");
    println!("  -> Adding a new indicator to the DB makes every cell Computed again,");
    println!("     but only for that one new indicator.");
}

// ============================================================
// SECTION 5: Total Failure - When a Partial Run Becomes an Error
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_total_failure() {
    println!("============================================================");
    println!("SECTION 5: Total Failure - When a Partial Run Becomes an Error");
    println!("============================================================");
    let summary = |computed, skipped, failed: &[&str]| BatchSummary {
        computed,
        skipped,
        failed_cells: failed.iter().map(|i| Cell::new(*i, 1)).collect(),
    };
    let cases = [
        ("all computed", summary(3, 0, &[])),
        ("partial failure", summary(1, 0, &["USDJPY"])),
        ("everything skipped", summary(0, 4, &[])),
        ("empty batch", summary(0, 0, &[])),
        ("all failed", summary(0, 0, &["EURUSD", "USDJPY"])),
        ("skips + only failures", summary(0, 5, &["EURUSD"])),
    ];
    for (label, s) in &cases {
        let verdict = fail_on_total_failure(s).map_or_else(|e| format!("Err: {e}"), |()| "Ok".into());
        println!("{label:<22} c={} s={} f={} -> {verdict}", s.computed, s.skipped, s.failed());
    }
    println!();
    println!("  -> Rule: failed > 0 AND computed == 0. Skips do not count as");
    println!("     progress: skipped cells were never attempted.");
    println!("  -> Isolated failures exit 0 and are retried next run (they are not");
    println!("     in the manifest). Only a run that made NO progress while trying");
    println!("     looks systemic (e.g. DB down mid-run) and fails the command.");
    println!("  -> Persist happens BEFORE this check, so a partial run's work is kept.");
}

// ============================================================
// SECTION 6: Parallel but Ordered - par_iter Keeps Cell Order
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_order() {
    println!("============================================================");
    println!("SECTION 6: Parallel but Ordered - par_iter Keeps Cell Order");
    println!("============================================================");
    let source = mirror_index(vec![rsi(7)]);
    let cells = source.cells();
    let no_manifest = FxHashMap::default();
    // Exactly compute_cells' shape: par_iter().map(compute_cell).collect().
    let outcomes: Vec<MirrorOutcome> = cells.par_iter().map(|c| mirror_compute_cell(&source, c, &no_manifest)).collect();
    let (computed, summary) = mirror_split(outcomes);
    println!("cells in      : {:?}", cells.iter().map(ToString::to_string).collect::<Vec<_>>());
    println!("computed out  : {:?}", computed.iter().map(|o| o.cell.to_string()).collect::<Vec<_>>());
    println!("failed_cells  : {:?}", summary.failed_cells.iter().map(ToString::to_string).collect::<Vec<_>>());
    println!("summary       : computed={} skipped={} failed={}", summary.computed, summary.skipped, summary.failed());
    println!();
    println!("  -> The 'error!' line above may print in any order (threads race),");
    println!("     but collect() on an indexed parallel iterator keeps input order,");
    println!("     so outputs and failed_cells are deterministic.");
    println!("  -> Outcomes are gathered first and split after: workers share only");
    println!("     &SourceIndex (read-only), no locks or atomic counters needed.");
    println!("  -> Real compute_cells also logs signal_group_cache_stats(): with many");
    println!("     mms per instrument, every cell after the first is a cache hit.");
}
