//! Deep Dive: strategy_adapter (2/2) — the id scan and the error paths
//! Source: Code input — src/infrastructure/strategy_adapter.rs
//!
//! max_strategy_id walks every partition under the base dir and reads only the
//! strategy_id column, so new ids stay unique across the whole tree. Every
//! failure (unreadable file, mistyped column, blocked directory) comes back as
//! an anyhow::Error whose context names the offending path or column, never a
//! panic: release builds use panic = "abort".

use std::env;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow::array::{ArrayRef, Int32Array, Int64Array, RecordBatch};
use backtesting_engine::domain::cell::Cell;
use backtesting_engine::domain::strategy::Strategy;
use backtesting_engine::infrastructure::strategy_adapter::{ParquetStrategyRepository, StrategyRow};
use parquet::arrow::ArrowWriter;
use tempfile::TempDir;

fn strategy(indicator_id: i32) -> Strategy {
    Strategy {
        instrument: "EURUSD".to_string(),
        indicator_id,
        annual_roi: 12.34,
        max_draw_down: 25.67,
        annual_operation_count: 15.2,
        long_operation_point_ids: vec![1, 2],
        short_operation_point_ids: vec![3],
        money_management_strategy_id: 390,
    }
}

fn rows(indicator_ids: &[i32], first_id: i64) -> Vec<StrategyRow> {
    indicator_ids.iter().zip(first_id..).map(|(&ind, id)| StrategyRow::from_strategy(strategy(ind), id)).collect()
}

fn partition(root: &Path, instrument: &str, mms_id: i32) -> PathBuf {
    root.join(format!("instrument={instrument}/money_management_strategy_id={mms_id}/strategies.parquet"))
}

// A hand-written file, as an older writer or another tool might leave behind.
fn write_foreign(path: &Path, columns: Vec<(&str, ArrayRef)>) {
    fs::create_dir_all(path.parent().expect("has parent")).expect("mkdir");
    let batch = RecordBatch::try_from_iter(columns).expect("well-formed");
    let mut w = ArrowWriter::try_new(File::create(path).expect("create"), batch.schema(), None).expect("writer");
    w.write(&batch).expect("write");
    w.close().expect("close");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_empty_trees(),
        "2" => section_2_scan_spans_partitions(),
        "3" => section_3_projection_by_name(),
        "4" => section_4_truncated_partition(),
        "5" => section_5_mistyped_columns(),
        "6" => section_6_blocked_directory(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-strategy-adapter-ids-errors <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Empty Trees: max_strategy_id before any write");
    println!("  2    - Scan Spans Partitions: global max, stray files ignored");
    println!("  3    - Projection By Name: only strategy_id is decoded");
    println!("  4    - Truncated Partition: errors that name the file");
    println!("  5    - Mistyped Columns: drifted schemas become Err, not abort");
    println!("  6    - Blocked Directory: create_dir_all failure context");
}

// ============================================================
// SECTION 1: Empty Trees - max_strategy_id before any write
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_empty_trees() {
    println!("============================================================");
    println!("SECTION 1: Empty Trees - max_strategy_id before any write");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");

    let missing = ParquetStrategyRepository::new(tmp.path().join("strategy"));
    println!("base dir missing       -> {:?}", missing.max_strategy_id().map_err(|e| e.to_string()));

    fs::create_dir_all(tmp.path().join("strategy/instrument=EURUSD")).expect("mkdir");
    let empty = ParquetStrategyRepository::new(tmp.path().join("strategy"));
    println!("dirs but no partitions -> {:?}", empty.max_strategy_id().map_err(|e| e.to_string()));
    println!();
    println!("  -> Persistence computes next_id = max + 1, so the first id is 1");
    println!("  -> The missing-dir check is an early return: no read_dir error");
}

// ============================================================
// SECTION 2: Scan Spans Partitions - global max, stray files ignored
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_scan_spans_partitions() {
    println!("============================================================");
    println!("SECTION 2: Scan Spans Partitions - global max, stray files ignored");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");
    let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
    repo.write_rows(&Cell::new("EURUSD", 390), rows(&[2001, 2002], 1)).expect("write");
    repo.write_rows(&Cell::new("USDJPY", 400), rows(&[2001, 2002], 7)).expect("write");
    repo.write_rows(&Cell::new("EURUSD", 391), rows(&[2001, 2002], 3)).expect("write");
    println!("ids written: EURUSD/390 [1,2], USDJPY/400 [7,8], EURUSD/391 [3,4]");

    // Only files named exactly strategies.parquet are scanned.
    let dir = tmp.path().join("instrument=EURUSD/money_management_strategy_id=390");
    fs::write(dir.join("notes.txt"), "not parquet").expect("write");
    fs::write(dir.join("strategies.parquet.bak"), "not parquet").expect("write");

    println!("max_strategy_id()  -> {:?}", repo.max_strategy_id().map_err(|e| e.to_string()));
    println!();
    println!("  -> Iterative DFS: a Vec<PathBuf> stack, no recursion");
    println!("  -> The max is global, not per cell: USDJPY's 8 wins");
    println!("  -> Cost grows with partition count, paid once per persist_cells");
}

// ============================================================
// SECTION 3: Projection By Name - only strategy_id is decoded
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_projection_by_name() {
    println!("============================================================");
    println!("SECTION 3: Projection By Name - only strategy_id is decoded");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");
    let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
    // strategy_id is the SECOND column here, unlike the writer's schema.
    write_foreign(
        &partition(tmp.path(), "EURUSD", 390),
        vec![
            ("indicator_id", Arc::new(Int64Array::from(vec![2001, 2002])) as ArrayRef),
            ("strategy_id", Arc::new(Int64Array::from(vec![7, 3])) as ArrayRef),
        ],
    );
    println!("file columns: [indicator_id, strategy_id], strategy_id = [7, 3]");
    println!("max_strategy_id() -> {:?}", repo.max_strategy_id().map_err(|e| e.to_string()));
    println!();
    println!("  -> schema().index_of(\"strategy_id\") finds the column by name");
    println!("  -> ProjectionMask::roots skips decoding the list and metric columns");
    println!("  -> The other columns are not validated at all on this path");
}

// ============================================================
// SECTION 4: Truncated Partition - errors that name the file
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_truncated_partition() {
    println!("============================================================");
    println!("SECTION 4: Truncated Partition - errors that name the file");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");
    let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
    let cell = Cell::new("EURUSD", 390);
    repo.write_rows(&cell, rows(&[2001], 1)).expect("write");
    // What a crash midway through the in-place rewrite could leave behind.
    fs::write(partition(tmp.path(), "EURUSD", 390), b"PAR1").expect("truncate");

    let root = tmp.path().display().to_string();
    let shorten = |e: anyhow::Error| format!("{e:#}").replace(&root, "<tmp>");
    println!("write_rows:");
    println!("  {}", shorten(repo.write_rows(&cell, rows(&[9999], 2)).unwrap_err()));
    println!("max_strategy_id:");
    println!("  {}", shorten(repo.max_strategy_id().unwrap_err()));
    println!();
    println!("  -> {{e:#}} prints the full context chain, outermost first");
    println!("  -> write_rows fails BEFORE touching the file: the bad file survives");
    println!("  -> Accepted risk: delete the partition and re-run; Postgres is the source");
}

// ============================================================
// SECTION 5: Mistyped Columns - drifted schemas become Err, not abort
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_mistyped_columns() {
    println!("============================================================");
    println!("SECTION 5: Mistyped Columns - drifted schemas become Err, not abort");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");
    let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
    let cell = Cell::new("EURUSD", 390);
    // indicator_id widened to Int64; the reader expects Int32.
    write_foreign(
        &partition(tmp.path(), "EURUSD", 390),
        vec![
            ("strategy_id", Arc::new(Int64Array::from(vec![1])) as ArrayRef),
            ("indicator_id", Arc::new(Int64Array::from(vec![2001])) as ArrayRef),
        ],
    );
    let err = repo.write_rows(&cell, rows(&[9999], 2)).unwrap_err();
    println!("write_rows (indicator_id Int64): {}", err.root_cause());
    // The scan tolerates it: it only looks at strategy_id.
    println!("max_strategy_id on same file   : {:?}", repo.max_strategy_id().map_err(|e| e.to_string()));

    // strategy_id narrowed to Int32: now the scan fails too.
    write_foreign(
        &partition(tmp.path(), "USDJPY", 400),
        vec![("strategy_id", Arc::new(Int32Array::from(vec![7])) as ArrayRef)],
    );
    println!("max_strategy_id (strategy_id Int32): {}", repo.max_strategy_id().unwrap_err().root_cause());
    println!();
    println!("  -> decode_batch goes through parquet_columns' checked downcasts");
    println!("  -> A missing column fails the same way: \"missing column <name>\"");
}

// ============================================================
// SECTION 6: Blocked Directory - create_dir_all failure context
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_blocked_directory() {
    println!("============================================================");
    println!("SECTION 6: Blocked Directory - create_dir_all failure context");
    println!("============================================================");
    let tmp = TempDir::new().expect("tempdir");
    let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
    // A FILE where the instrument= directory should be.
    fs::write(tmp.path().join("instrument=EURUSD"), b"").expect("write");

    let root = tmp.path().display().to_string();
    let err = repo.write_rows(&Cell::new("EURUSD", 390), rows(&[2001], 1)).unwrap_err();
    for (depth, cause) in err.chain().enumerate() {
        println!("  [{depth}] {}", cause.to_string().replace(&root, "<tmp>"));
    }
    println!();
    println!("  -> path.exists() is false, so there is nothing to read: no read error");
    println!("  -> The `let Some(parent) else bail!` guard never fires for real paths;");
    println!("     partition_path always ends in <dir>/strategies.parquet");
}
