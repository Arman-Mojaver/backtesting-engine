//! Deep Dive: StrategyPersistence (1/2) — strategy_id assignment
//! Source: Code input — src/infrastructure/strategy_persistence.rs
//!
//! StrategyPersistence is the single write entry point used by
//! `bte strategy create` and `bte strategy create-loop`. It owns two stores
//! under one parquet root: the partitioned strategy tree (`strategy/`) and the
//! processed manifest (`processed.parquet`). This script covers how
//! persist_cells assigns strategy_id: sequentially, from the GLOBAL max across
//! every partition, BEFORE the parallel per-partition writes start.
//! Script 2/2 covers the manifest and the failure ordering.

use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};

use arrow::array::AsArray;
use arrow::datatypes::{Int32Type, Int64Type};
use backtesting_engine::application::cell_output::CellOutput;
use backtesting_engine::domain::cell::Cell;
use backtesting_engine::domain::strategy::Strategy;
use backtesting_engine::infrastructure::strategy_adapter::{ParquetStrategyRepository, StrategyRow};
use backtesting_engine::infrastructure::strategy_persistence::StrategyPersistence;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use tempfile::TempDir;

// ---- Seed data ----
fn strategy(instrument: &str, mms_id: i32, indicator_id: i32) -> Strategy {
    Strategy {
        instrument: instrument.to_string(),
        indicator_id,
        annual_roi: 12.34,
        max_draw_down: 25.67,
        annual_operation_count: 15.2,
        long_operation_point_ids: vec![1, 2],
        short_operation_point_ids: vec![3],
        money_management_strategy_id: mms_id,
    }
}

// One strategy per indicator: every computed indicator produced a strategy.
fn cell(instrument: &str, mms_id: i32, indicator_ids: &[i32]) -> CellOutput {
    CellOutput {
        cell: Cell::new(instrument, mms_id),
        strategies: indicator_ids.iter().map(|&i| strategy(instrument, mms_id, i)).collect(),
        computed_indicator_ids: indicator_ids.to_vec(),
    }
}

fn setup() -> (TempDir, StrategyPersistence) {
    let tmp = TempDir::new().expect("tempdir");
    let persistence = StrategyPersistence::new(tmp.path().to_path_buf());
    (tmp, persistence)
}

fn partition(root: &Path, instrument: &str, mms_id: i32) -> PathBuf {
    root.join(format!("strategy/instrument={instrument}/money_management_strategy_id={mms_id}/strategies.parquet"))
}

// No public reader exists for strategies, so read (strategy_id, indicator_id) directly.
fn ids(root: &Path, instrument: &str, mms_id: i32) -> Vec<(i64, i32)> {
    let file = File::open(partition(root, instrument, mms_id)).expect("partition exists");
    let reader = ParquetRecordBatchReaderBuilder::try_new(file).expect("parquet").build().expect("reader");
    let mut out = Vec::new();
    for batch in reader {
        let batch = batch.expect("readable");
        let sids = batch.column_by_name("strategy_id").expect("col").as_primitive::<Int64Type>();
        let inds = batch.column_by_name("indicator_id").expect("col").as_primitive::<Int32Type>();
        out.extend(sids.values().iter().copied().zip(inds.values().iter().copied()));
    }
    out
}

fn show(root: &Path, instrument: &str, mms_id: i32) {
    let pairs: Vec<String> = ids(root, instrument, mms_id).iter().map(|(s, i)| format!("{s}<-{i}")).collect();
    println!("  {instrument}/{mms_id}: [{}]  (strategy_id<-indicator_id)", pairs.join(", "));
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_one_root_two_stores(),
        "2" => section_2_global_id_sequence(),
        "3" => section_3_reserved_ids_leave_gaps(),
        "4" => section_4_saturating_at_i64_max(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-strategy-persistence-ids <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - One Root, Two Stores: strategy tree + processed manifest");
    println!("  2    - Global Id Sequence: ids continue across cells and runs");
    println!("  3    - Reserved Ids Leave Gaps: re-persisting burns ids");
    println!("  4    - Saturating At i64 Max: ids stop growing and repeat");
}

// ============================================================
// SECTION 1: One Root, Two Stores - strategy tree + processed manifest
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_one_root_two_stores() {
    println!("============================================================");
    println!("SECTION 1: One Root, Two Stores - strategy tree + processed manifest");
    println!("============================================================");
    let (tmp, persistence) = setup();
    let debug = format!("{persistence:?}").replace(&tmp.path().display().to_string(), "<root>");
    println!("{debug}");
    println!();

    persistence.persist_cells(vec![cell("EURUSD", 390, &[2001]), cell("USDJPY", 400, &[3001])]).expect("persist");

    let mut files = Vec::new();
    walk(tmp.path(), &mut files);
    files.sort();
    for f in files {
        println!("  {}", f.strip_prefix(tmp.path()).expect("under root").display());
    }
    println!();
    println!("  -> new() joins STRATEGY_DIR_NAME (\"strategy\") for the repository");
    println!("  -> The manifest sits at the root, NEXT TO strategy/, not inside it,");
    println!("     so max_strategy_id's tree scan never opens processed.parquet");
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable").flatten() {
        let p = entry.path();
        if p.is_dir() { walk(&p, out); } else { out.push(p); }
    }
}

// ============================================================
// SECTION 2: Global Id Sequence - ids continue across cells and runs
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_global_id_sequence() {
    println!("============================================================");
    println!("SECTION 2: Global Id Sequence - ids continue across cells and runs");
    println!("============================================================");
    let (tmp, persistence) = setup();
    let root = tmp.path();

    // Run 1: two cells in ONE call. next_id starts at max(0) + 1 = 1 and
    // walks the cells in input order, before any parallel write begins.
    persistence.persist_cells(vec![cell("EURUSD", 390, &[2001, 2002]), cell("USDJPY", 400, &[2001, 2002])]).expect("run 1");
    println!("run 1 (EURUSD/390 + USDJPY/400 together):");
    show(root, "EURUSD", 390);
    show(root, "USDJPY", 400);

    // Run 2: a brand-new cell. Its own partition is empty, but ids continue
    // from the max over the WHOLE tree (4), not from its partition (0).
    persistence.persist_cells(vec![cell("GBPUSD", 1, &[2001])]).expect("run 2");
    println!("run 2 (GBPUSD/1 alone):");
    show(root, "GBPUSD", 1);
    println!();
    println!("  -> Same indicator 2001 in 3 cells -> 3 distinct strategy_ids (1, 3, 5)");
    println!("  -> strategy_id is unique across the tree, so a query over all");
    println!("     partitions can use it as a key without the cell columns");
    println!("  -> Ids are fixed before into_par_iter(): write order can't change them");
}

// ============================================================
// SECTION 3: Reserved Ids Leave Gaps - re-persisting burns ids
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_reserved_ids_leave_gaps() {
    println!("============================================================");
    println!("SECTION 3: Reserved Ids Leave Gaps - re-persisting burns ids");
    println!("============================================================");
    let (tmp, persistence) = setup();
    let root = tmp.path();
    persistence.persist_cells(vec![cell("EURUSD", 390, &[2001, 2002])]).expect("first");
    println!("first run:");
    show(root, "EURUSD", 390);

    // EURUSD/390 again (indicators already stored) plus a new USDJPY cell.
    // persist_cells numbers EURUSD as 3, 4 and USDJPY as 5 — it does not
    // know write_rows will drop EURUSD's rows as duplicate indicator_ids.
    persistence.persist_cells(vec![cell("EURUSD", 390, &[2001, 2002]), cell("USDJPY", 400, &[3001])]).expect("second");
    println!("second run (EURUSD/390 repeated + USDJPY/400):");
    show(root, "EURUSD", 390);
    show(root, "USDJPY", 400);
    println!();
    println!("  -> EURUSD keeps 1, 2: write_rows dedups by indicator_id (existing wins)");
    println!("  -> USDJPY gets 5, not 3: ids 3 and 4 were assigned, then discarded");
    println!("  -> Ids are unique and increasing, but NOT dense");
    println!("  -> In practice the manifest filters processed indicators out first,");
    println!("     so gaps only appear after a failed run (see script 2, section 3)");
}

// ============================================================
// SECTION 4: Saturating At i64 Max - ids stop growing and repeat
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_saturating_at_i64_max() {
    println!("============================================================");
    println!("SECTION 4: Saturating At i64 Max - ids stop growing and repeat");
    println!("============================================================");
    let (tmp, persistence) = setup();
    let root = tmp.path();

    // Plant a row with strategy_id = i64::MAX straight through the repository.
    let repo = ParquetStrategyRepository::new(root.join("strategy"));
    let planted = StrategyRow::from_strategy(strategy("EURUSD", 390, 2001), i64::MAX);
    repo.write_rows(&Cell::new("EURUSD", 390), vec![planted]).expect("plant");
    println!("planted EURUSD/390 with strategy_id = i64::MAX");

    // The crate denies arithmetic_side_effects, so `+ 1` is spelled
    // saturating_add(1): no panic, no wrap to negative — it sticks at MAX.
    persistence.persist_cells(vec![cell("USDJPY", 400, &[3001, 3002])]).expect("persist");
    show(root, "USDJPY", 400);
    println!();
    let max = i64::MAX;
    println!("  -> All three rows now share strategy_id {max}");
    println!("  -> Unreachable in practice (9.2e18 strategies); saturating_add is");
    println!("     there to satisfy the lint, not as a real overflow policy");
}
