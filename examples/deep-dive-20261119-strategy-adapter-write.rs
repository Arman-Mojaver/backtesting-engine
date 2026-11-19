//! Deep Dive: strategy_adapter (1/2) — the write path
//! Source: Code input — src/infrastructure/strategy_adapter.rs
//!
//! ParquetStrategyRepository stores strategies as one Parquet file per
//! (instrument, money_management_strategy_id) cell, hive-style. write_rows is
//! read-merge-rewrite: the whole partition is read, incoming rows whose
//! indicator_id already exists are dropped, and the file is rewritten in place.
//! Metrics are stored as Decimal128(10, 2), not f64.
//! Caller: strategy_persistence.rs (assigns strategy_id, then writes in parallel).

use std::env;
use std::fs::File;
use std::path::Path;

use arrow::array::{Array, AsArray, RecordBatch};
use arrow::datatypes::{Decimal128Type, Int32Type, Int64Type};
use backtesting_engine::domain::cell::Cell;
use backtesting_engine::domain::strategy::Strategy;
use backtesting_engine::infrastructure::strategy_adapter::{ParquetStrategyRepository, StrategyRow};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use tempfile::TempDir;

// ---- Seed data: two EURUSD strategies under money-management strategy 390 ----
fn strategy(indicator_id: i32, annual_roi: f64, long: Vec<i32>, short: Vec<i32>) -> Strategy {
    Strategy {
        instrument: "EURUSD".to_string(),
        indicator_id,
        annual_roi,
        max_draw_down: 25.67,
        annual_operation_count: 15.2,
        long_operation_point_ids: long,
        short_operation_point_ids: short,
        money_management_strategy_id: 390,
    }
}

fn seed() -> Vec<Strategy> {
    vec![strategy(2001, 12.34, vec![1, 2, 3], vec![4, 5]), strategy(2002, -5.10, vec![6], vec![])]
}

// Mirrors strategy_persistence: ids are contiguous from `first_id`.
fn rows(strategies: Vec<Strategy>, first_id: i64) -> Vec<StrategyRow> {
    strategies.into_iter().zip(first_id..).map(|(s, id)| StrategyRow::from_strategy(s, id)).collect()
}

// The repository has no public reader, so read the file with parquet directly.
fn read_batch(path: &Path) -> RecordBatch {
    let file = File::open(path).expect("partition exists");
    let mut reader = ParquetRecordBatchReaderBuilder::try_new(file).expect("parquet").build().expect("reader");
    reader.next().expect("one batch").expect("readable")
}

fn eurusd_390(root: &Path) -> std::path::PathBuf {
    root.join("instrument=EURUSD/money_management_strategy_id=390/strategies.parquet")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_from_strategy(),
        "2" => section_2_partition_layout(),
        "3" => section_3_decimal_metrics(),
        "4" => section_4_dedup_by_indicator_id(),
        "5" => section_5_nothing_fresh(),
        "6" => section_6_list_columns(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-strategy-adapter-write <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - From Strategy: what StrategyRow keeps and drops");
    println!("  2    - Partition Layout: one hive-style file per cell");
    println!("  3    - Decimal Metrics: f64 -> Decimal128(10, 2) and back");
    println!("  4    - Dedup By Indicator Id: existing rows win, ids get gaps");
    println!("  5    - Nothing Fresh: the early return that skips the rewrite");
    println!("  6    - List Columns: operation point ids as List<Int32>");
}

// ============================================================
// SECTION 1: From Strategy - what StrategyRow keeps and drops
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_from_strategy() {
    println!("============================================================");
    println!("SECTION 1: From Strategy - what StrategyRow keeps and drops");
    println!("============================================================");
    let s = seed().remove(0);
    println!("Strategy.instrument = {:?}", s.instrument);

    // The id comes from the caller, not the strategy: persistence assigns it
    // from max_strategy_id() + 1 across the WHOLE tree (see script 2).
    let row = StrategyRow::from_strategy(s, 42);
    println!("{row:?}");
    println!();
    println!("  -> instrument is gone: the partition path already encodes it");
    println!("  -> money_management_strategy_id is kept as a column AND in the path");
}

// ============================================================
// SECTION 2: Partition Layout - one hive-style file per cell
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_partition_layout() {
    println!("============================================================");
    println!("SECTION 2: Partition Layout - one hive-style file per cell");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");
    let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());

    // write_rows creates missing directories (create_dir_all) on demand.
    repo.write_rows(&Cell::new("EURUSD", 390), rows(seed(), 1)).expect("write");
    repo.write_rows(&Cell::new("EURUSD", 391), rows(seed(), 3)).expect("write");
    repo.write_rows(&Cell::new("USDJPY", 400), rows(seed(), 5)).expect("write");

    let mut files: Vec<String> = walk(tmp.path())
        .iter()
        .map(|p| p.strip_prefix(tmp.path()).expect("under root").display().to_string())
        .collect();
    files.sort();
    for f in files {
        println!("  {f}");
    }
    println!();
    println!("  -> key=value directories: DuckDB/Polars/Spark read them as columns");
    println!("  -> Partitions are independent, so persistence writes them with rayon");
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir).expect("readable").flatten() {
        let p = entry.path();
        if p.is_dir() { out.extend(walk(&p)); } else { out.push(p); }
    }
    out
}

// ============================================================
// SECTION 3: Decimal Metrics - f64 -> Decimal128(10, 2) and back
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_decimal_metrics() {
    println!("============================================================");
    println!("SECTION 3: Decimal Metrics - f64 -> Decimal128(10, 2) and back");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");
    let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
    // metric_array: round(v * 100) half away from zero, stored as i128.
    let inputs = [12.346, 1.005, 0.004, -0.005, 123_456_789.0];
    let strategies = inputs.iter().zip(3000..).map(|(&roi, id)| strategy(id, roi, vec![], vec![])).collect();
    repo.write_rows(&Cell::new("EURUSD", 390), rows(strategies, 1)).expect("write");

    let batch = read_batch(&eurusd_390(tmp.path()));
    let col = batch.column_by_name("annual_roi").expect("present");
    println!("on-disk type: {}", col.data_type());
    let raw = col.as_primitive::<Decimal128Type>();
    for (input, stored) in inputs.iter().zip(raw.values()) {
        // metric_from_decimal: raw / 100 on the way back.
        println!("  {input:>11} -> raw {stored:>11} -> read back {}", *stored as f64 / 100.0);
    }
    println!();
    println!("  -> 1.005 is 1.00499.. in binary, so *100 rounds DOWN to 100");
    println!("  -> -0.005 rounds half AWAY from zero: -1, not 0");
    println!("  -> 123456789 needs 11 digits > precision 10, yet writes and reads");
    println!("     back intact: nothing enforces precision (accepted, see 84354d5)");
}

// ============================================================
// SECTION 4: Dedup By Indicator Id - existing rows win, ids get gaps
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_dedup_by_indicator_id() {
    println!("============================================================");
    println!("SECTION 4: Dedup By Indicator Id - existing rows win, ids get gaps");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");
    let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
    let cell = Cell::new("EURUSD", 390);
    repo.write_rows(&cell, rows(seed(), 1)).expect("first write");

    // Second batch: 2002 again (with a different ROI) plus a new 3000.
    let second = vec![strategy(2002, 99.0, vec![100], vec![]), strategy(3000, 7.5, vec![], vec![])];
    println!("incoming: (id 3, ind 2002, roi 99.0), (id 4, ind 3000, roi 7.5)");
    repo.write_rows(&cell, rows(second, 3)).expect("second write");

    let batch = read_batch(&eurusd_390(tmp.path()));
    let ids = batch.column_by_name("strategy_id").expect("col").as_primitive::<Int64Type>();
    let inds = batch.column_by_name("indicator_id").expect("col").as_primitive::<Int32Type>();
    let rois = batch.column_by_name("annual_roi").expect("col").as_primitive::<Decimal128Type>();
    for i in 0..batch.num_rows() {
        println!("  strategy_id={} indicator_id={} roi={}", ids.value(i), inds.value(i), rois.value_as_string(i));
    }
    println!();
    println!("  -> 2002 keeps id 2 and roi -5.10: the incoming duplicate is dropped");
    println!("  -> id 3 was assigned upstream but never stored: ids can have gaps");
    println!("  -> Order: existing rows first, fresh rows appended");
}

// ============================================================
// SECTION 5: Nothing Fresh - the early return that skips the rewrite
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_nothing_fresh() {
    println!("============================================================");
    println!("SECTION 5: Nothing Fresh - the early return that skips the rewrite");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");
    let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
    let cell = Cell::new("EURUSD", 390);
    repo.write_rows(&cell, rows(seed(), 1)).expect("first write");
    let path = eurusd_390(tmp.path());
    let before = std::fs::metadata(&path).expect("meta").modified().expect("mtime");

    std::thread::sleep(std::time::Duration::from_millis(20));
    repo.write_rows(&cell, rows(seed(), 3)).expect("same rows again");
    let after = std::fs::metadata(&path).expect("meta").modified().expect("mtime");
    println!("same indicator ids again -> file modified? {}", before != after);

    // Empty input takes the same branch, and on a MISSING partition it does not
    // even create the directory.
    repo.write_rows(&Cell::new("GBPUSD", 1), vec![]).expect("empty");
    println!("empty rows on new cell   -> dir created? {}", tmp.path().join("instrument=GBPUSD").exists());
    println!();
    println!("  -> fresh_rows.is_empty() returns Ok(()) before any write");
    println!("  -> Re-running a command over processed cells is cheap and safe");
    println!("  -> The existing file is still fully READ to compute existing_ids");
}

// ============================================================
// SECTION 6: List Columns - operation point ids as List<Int32>
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_list_columns() {
    println!("============================================================");
    println!("SECTION 6: List Columns - operation point ids as List<Int32>");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");
    let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
    repo.write_rows(&Cell::new("EURUSD", 390), rows(seed(), 1)).expect("write");
    let batch = read_batch(&eurusd_390(tmp.path()));

    for name in ["long_operation_point_ids", "short_operation_point_ids"] {
        let list = batch.column_by_name(name).expect("col").as_list::<i32>();
        println!("{name}: {}", list.data_type());
        for i in 0..list.len() {
            let items = list.value(i);
            println!("  row {i}: {:?}", items.as_primitive::<Int32Type>().values().to_vec());
        }
    }
    println!();
    println!("  -> build_list_column: append_slice(items) + append(true) per row");
    println!("  -> Empty Vec -> empty list (not null): row 1 short ids above");
    println!("  -> Item field is nullable only because ListBuilder's default is;");
    println!("     the schema in build_record_batch must match it exactly");
}
