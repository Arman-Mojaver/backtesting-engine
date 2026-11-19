//! Deep Dive: StrategyPersistence (2/2) — the manifest and failure ordering
//! Source: Code input — src/infrastructure/strategy_persistence.rs
//!
//! persist_cells runs in three steps, and the order is the point:
//!   1. assign strategy_ids (script 1/2)
//!   2. write every strategy partition in parallel (rayon try_for_each)
//!   3. ONLY IF all writes succeeded: append one ManifestEntry per cell
//! The manifest is what makes `bte strategy create*` resumable: the commands
//! call read_processed_ids_per_cell() first and skip indicators listed there.

use std::env;
use std::fs::File;
use std::path::{Path, PathBuf};

use arrow::array::AsArray;
use arrow::datatypes::{Int32Type, Int64Type};
use backtesting_engine::application::cell_output::CellOutput;
use backtesting_engine::domain::cell::Cell;
use backtesting_engine::domain::strategy::Strategy;
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

// `with_strategy` lists the indicators that produced a strategy; `computed`
// lists every indicator that was run. They differ when an indicator yields none.
fn cell(instrument: &str, mms_id: i32, with_strategy: &[i32], computed: &[i32]) -> CellOutput {
    CellOutput {
        cell: Cell::new(instrument, mms_id),
        strategies: with_strategy.iter().map(|&i| strategy(instrument, mms_id, i)).collect(),
        computed_indicator_ids: computed.to_vec(),
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

fn strategy_ids(root: &Path, instrument: &str, mms_id: i32) -> Vec<i64> {
    let file = File::open(partition(root, instrument, mms_id)).expect("partition exists");
    let reader = ParquetRecordBatchReaderBuilder::try_new(file).expect("parquet").build().expect("reader");
    reader
        .flat_map(|b| b.expect("readable").column_by_name("strategy_id").expect("col").as_primitive::<Int64Type>().values().to_vec())
        .collect()
}

fn processed(persistence: &StrategyPersistence) -> Vec<String> {
    let map = persistence.read_processed_ids_per_cell().expect("manifest readable");
    let mut lines: Vec<String> = map
        .into_iter()
        .map(|(cell, ids)| {
            let mut ids: Vec<i32> = ids.into_iter().collect();
            ids.sort_unstable();
            format!("{cell}: {ids:?}")
        })
        .collect();
    lines.sort();
    lines
}

// Read processed.parquet directly to see the processed_at column.
fn processed_at(root: &Path) -> Vec<(i32, i64)> {
    let file = File::open(root.join("processed.parquet")).expect("manifest exists");
    let reader = ParquetRecordBatchReaderBuilder::try_new(file).expect("parquet").build().expect("reader");
    let mut out = Vec::new();
    for batch in reader {
        let batch = batch.expect("readable");
        let inds = batch.column_by_name("indicator_id").expect("col").as_primitive::<Int32Type>();
        let ats = batch.column_by_name("processed_at").expect("col").as_primitive::<Int64Type>();
        out.extend(inds.values().iter().copied().zip(ats.values().iter().copied()));
    }
    out
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_computed_not_stored(),
        "2" => section_2_empty_input_vs_empty_cell(),
        "3" => section_3_failed_write_skips_manifest(),
        "4" => section_4_corrupt_partition_blocks_everything(),
        "5" => section_5_one_timestamp_per_batch(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-strategy-persistence-manifest <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Computed Not Stored: the manifest lists every run indicator");
    println!("  2    - Empty Input Vs Empty Cell: the early return and its limit");
    println!("  3    - Failed Write Skips Manifest: and how a rerun heals it");
    println!("  4    - Corrupt Partition Blocks Everything: the global id scan");
    println!("  5    - One Timestamp Per Batch: processed_at and re-persisting");
}

// ============================================================
// SECTION 1: Computed Not Stored - the manifest lists every run indicator
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_computed_not_stored() {
    println!("============================================================");
    println!("SECTION 1: Computed Not Stored - the manifest lists every run indicator");
    println!("============================================================");
    let (tmp, persistence) = setup();
    // Indicators 2001..2003 were run; only 2001 and 2002 produced a strategy.
    persistence.persist_cells(vec![cell("EURUSD", 390, &[2001, 2002], &[2001, 2002, 2003])]).expect("persist");

    println!("strategy ids stored : {:?}", strategy_ids(tmp.path(), "EURUSD", 390));
    println!("manifest            :");
    for line in processed(&persistence) {
        println!("  {line}");
    }
    println!();
    println!("  -> ManifestEntry.indicator_ids = computed_indicator_ids, NOT the");
    println!("     strategies' indicator ids: 2003 is marked processed anyway");
    println!("  -> Without this, every rerun would recompute 2003 and find nothing");
    println!("  -> read_processed_ids_per_cell just delegates to the manifest");
}

// ============================================================
// SECTION 2: Empty Input Vs Empty Cell - the early return and its limit
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_empty_input_vs_empty_cell() {
    println!("============================================================");
    println!("SECTION 2: Empty Input Vs Empty Cell - the early return and its limit");
    println!("============================================================");
    let (tmp, persistence) = setup();
    persistence.persist_cells(Vec::new()).expect("empty");
    let entries = std::fs::read_dir(tmp.path()).expect("readable").count();
    println!("persist_cells(vec![])            -> files under root: {entries}");

    // A cell whose indicators all produced NO strategy.
    persistence.persist_cells(vec![cell("USDJPY", 400, &[], &[3001, 3002])]).expect("no strategies");
    println!("persist_cells(cell, 0 strategies):");
    println!("  strategy partition exists? {}", partition(tmp.path(), "USDJPY", 400).exists());
    println!("  processed.parquet exists?  {}", tmp.path().join("processed.parquet").exists());
    for line in processed(&persistence) {
        println!("  {line}");
    }
    println!();
    println!("  -> cells.is_empty() returns before max_strategy_id's tree scan");
    println!("  -> A non-empty cell with 0 rows still goes through the full path:");
    println!("     write_rows skips the file, but the manifest records both ids");
}

// ============================================================
// SECTION 3: Failed Write Skips Manifest - and how a rerun heals it
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_failed_write_skips_manifest() {
    println!("============================================================");
    println!("SECTION 3: Failed Write Skips Manifest - and how a rerun heals it");
    println!("============================================================");
    let (tmp, persistence) = setup();
    let root = tmp.path();
    // A FILE named instrument=EURUSD makes create_dir_all fail for EURUSD only.
    let blocker = root.join("strategy/instrument=EURUSD");
    std::fs::create_dir_all(blocker.parent().expect("parent")).expect("mkdir");
    std::fs::write(&blocker, b"").expect("blocker");
    let batch = || vec![cell("EURUSD", 390, &[2001], &[2001]), cell("USDJPY", 400, &[3001], &[3001])];

    let err = persistence.persist_cells(batch()).expect_err("EURUSD write fails");
    println!("run 1 error: {}", err);
    println!("  manifest cells: {}", processed(&persistence).len());
    // rayon does not roll back siblings: USDJPY may already be on disk.
    let usd_written = partition(root, "USDJPY", 400).exists();
    println!("  USDJPY partition on disk anyway? {usd_written}");

    std::fs::remove_file(&blocker).expect("unblock");
    persistence.persist_cells(batch()).expect("run 2");
    println!("run 2 (blocker removed, same batch): ok");
    println!("  EURUSD ids {:?}, USDJPY ids {:?}", strategy_ids(root, "EURUSD", 390), strategy_ids(root, "USDJPY", 400));
    println!("  manifest cells: {}", processed(&persistence).len());
    println!();
    println!("  -> Manifest is all-or-nothing per call; partitions are not");
    println!("  -> Rerun recomputes both cells; write_rows' indicator_id dedup keeps");
    println!("     USDJPY's run-1 row (id 2) and drops the new one (id 4)");
    println!("  -> EURUSD lands as id 3: ids 1 and 4 are the gaps of script 1 §3");
}

// ============================================================
// SECTION 4: Corrupt Partition Blocks Everything - the global id scan
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_corrupt_partition_blocks_everything() {
    println!("============================================================");
    println!("SECTION 4: Corrupt Partition Blocks Everything - the global id scan");
    println!("============================================================");
    let (tmp, persistence) = setup();
    let root = tmp.path();
    persistence.persist_cells(vec![cell("EURUSD", 390, &[2001], &[2001])]).expect("seed");
    // Truncate EURUSD's file to just the magic bytes.
    std::fs::write(partition(root, "EURUSD", 390), b"PAR1").expect("corrupt");

    // Persist an UNRELATED cell. It never touches EURUSD's partition...
    let err = persistence.persist_cells(vec![cell("USDJPY", 400, &[3001], &[3001])]).expect_err("fails");
    let chain = format!("{err:#}");
    let rel = chain.replace(&root.display().to_string(), "<root>");
    for part in rel.split(": ") {
        println!("  {part}");
    }
    println!("USDJPY written? {}", partition(root, "USDJPY", 400).exists());
    println!("manifest still readable: {:?}", processed(&persistence));
    println!();
    println!("  -> ...but max_strategy_id() opens EVERY strategies.parquet first,");
    println!("     so one bad file anywhere stops all persists before any write");
    println!("  -> The error names the bad file (with_context in max_strategy_id)");
    println!("  -> The manifest is a separate file and is unaffected");
}

// ============================================================
// SECTION 5: One Timestamp Per Batch - processed_at and re-persisting
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_one_timestamp_per_batch() {
    println!("============================================================");
    println!("SECTION 5: One Timestamp Per Batch - processed_at and re-persisting");
    println!("============================================================");
    let (tmp, persistence) = setup();
    let root = tmp.path();
    persistence
        .persist_cells(vec![cell("EURUSD", 390, &[2001], &[2001, 2002]), cell("USDJPY", 400, &[3001], &[3001])])
        .expect("run 1");
    println!("run 1 (indicator_id, processed_at): {:?}", processed_at(root));

    // unix_now_secs has 1-second resolution: wait so run 2 gets a new stamp.
    std::thread::sleep(std::time::Duration::from_millis(1100));
    persistence.persist_cells(vec![cell("EURUSD", 390, &[], &[2002])]).expect("run 2");
    println!("run 2 re-marks 2002 only:            {:?}", processed_at(root));
    println!();
    println!("  -> unix_now_secs() is called ONCE, after all writes: every row");
    println!("     of a batch shares one processed_at, regardless of cell");
    println!("  -> Re-appending an id replaces its row (no duplicate), with the");
    println!("     newer processed_at, and moves it to the end of the file");
}
