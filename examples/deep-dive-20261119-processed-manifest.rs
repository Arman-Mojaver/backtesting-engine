//! Deep Dive: ParquetProcessedManifest — the "already computed" ledger
//! Source: Code input — src/infrastructure/processed_manifest.rs
//!
//! `bte strategy create*` is resumable: before computing a cell it asks the
//! manifest which indicator ids were already processed for that
//! (instrument, mms_id) cell and skips them. After a batch is persisted,
//! StrategyPersistence::persist_cells appends one ManifestEntry per cell.
//! The whole ledger is ONE file, `<parquet_root>/processed.parquet`, with rows
//! (instrument, mms_id, indicator_id, processed_at). Every append rewrites it.
//!
//! The private helpers (merge_entries, write_atomically, ProcessedRow) cannot
//! be called from an example, so this script observes them through the file:
//! `dump_rows` reads processed.parquet directly to show exact on-disk rows.

use std::env;
use std::fs::{self, File};
use std::path::Path;
use std::sync::Arc;

use arrow::array::{ArrayRef, AsArray, Int32Array, Int64Array, StringArray};
use arrow::datatypes::{Int32Type, Int64Type};
use arrow::record_batch::RecordBatch;
use backtesting_engine::domain::cell::Cell;
use backtesting_engine::infrastructure::processed_manifest::{
    ManifestEntry, ParquetProcessedManifest, unix_now_secs,
};
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use tempfile::TempDir;

// ---- Seed data ----
const RUN_1_AT: i64 = 1_763_510_400; // 2025-11-19 00:00 UTC
const RUN_2_AT: i64 = 1_763_596_800; // one day later

fn entry(instrument: &str, mms_id: i32, ids: &[i32]) -> ManifestEntry {
    ManifestEntry { cell: Cell::new(instrument, mms_id), indicator_ids: ids.to_vec() }
}

fn fresh_manifest() -> (TempDir, ParquetProcessedManifest) {
    let tmp = TempDir::new().expect("tempdir");
    let manifest = ParquetProcessedManifest::new(tmp.path().to_path_buf());
    (tmp, manifest)
}

/// Reads processed.parquet the same way the private read_parquet does.
fn dump_rows(root: &Path) {
    let file = File::open(root.join("processed.parquet")).expect("manifest exists");
    let reader = ParquetRecordBatchReaderBuilder::try_new(file).expect("parquet").build().expect("reader");
    for batch in reader {
        let b = batch.expect("batch");
        let inst = b.column_by_name("instrument").expect("col").as_string::<i32>();
        let mms = b.column_by_name("mms_id").expect("col").as_primitive::<Int32Type>();
        let ind = b.column_by_name("indicator_id").expect("col").as_primitive::<Int32Type>();
        let at = b.column_by_name("processed_at").expect("col").as_primitive::<Int64Type>();
        for i in 0..b.num_rows() {
            let run = if at.value(i) == RUN_1_AT { "run 1" } else { "run 2" };
            println!("  {:<6} mms={} ind={} processed_at={} ({run})", inst.value(i), mms.value(i), ind.value(i), at.value(i));
        }
    }
}

fn list_dir(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(root).expect("read_dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().to_string()).collect();
    names.sort();
    names
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_missing_file(),
        "2" => section_2_first_append(),
        "3" => section_3_rerun_upsert(),
        "4" => section_4_batch_dedup_and_empty(),
        "5" => section_5_skip_lookup(),
        "6" => section_6_atomic_write(),
        "7" => section_7_error_paths(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-processed-manifest <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Missing File: First Run Reads an Empty Ledger");
    println!("  2    - First Append: Entries Flatten Into One Row per Indicator");
    println!("  3    - Rerun Upsert: Overlapping Ids Replace Old Rows");
    println!("  4    - Batch Dedup and Empty Entries");
    println!("  5    - Skip Lookup: How Callers Use the Per-Cell Map");
    println!("  6    - Atomic Write: Temp File, Rename, Cleanup");
    println!("  7    - Error Paths: Unopenable, Corrupt, Drifted Schema");
}

// ============================================================
// SECTION 1: Missing File - First Run Reads an Empty Ledger
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_missing_file() {
    println!("============================================================");
    println!("SECTION 1: Missing File - First Run Reads an Empty Ledger");
    println!("============================================================");
    let (tmp, manifest) = fresh_manifest();
    println!("root dir contents: {:?}", list_dir(tmp.path()));
    // read_all matches ErrorKind::NotFound and returns Ok(vec![]): a missing
    // ledger means "nothing processed yet", not an error.
    let map = manifest.read_processed_ids_per_cell().expect("empty ok");
    println!("read_processed_ids_per_cell() -> {} cells", map.len());
    println!("  -> File::open hit NotFound -> treated as empty ledger");
    println!("  -> reading never creates the file: {:?}", list_dir(tmp.path()));
}

// ============================================================
// SECTION 2: First Append - Entries Flatten Into One Row per Indicator
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_first_append() {
    println!("============================================================");
    println!("SECTION 2: First Append - Entries Flatten Into One Row per Indicator");
    println!("============================================================");
    let (tmp, manifest) = fresh_manifest();
    let entries = [entry("EURUSD", 390, &[2001, 2002]), entry("USDJPY", 400, &[3001])];
    println!("append_all(2 entries, processed_at={RUN_1_AT})");
    manifest.append_all(&entries, RUN_1_AT).expect("append");
    println!("root dir contents: {:?}", list_dir(tmp.path()));
    println!("On-disk rows (schema: Utf8, Int32, Int32, Int64; ZSTD):");
    dump_rows(tmp.path());
    // One processed_at per append call: the caller stamps the whole batch
    // with unix_now_secs() once, so every row of a run shares a timestamp.
    println!("  -> each (cell, indicator_id) pair became its own row");
}

// ============================================================
// SECTION 3: Rerun Upsert - Overlapping Ids Replace Old Rows
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_rerun_upsert() {
    println!("============================================================");
    println!("SECTION 3: Rerun Upsert - Overlapping Ids Replace Old Rows");
    println!("============================================================");
    let (tmp, manifest) = fresh_manifest();
    manifest.append_all(&[entry("EURUSD", 390, &[2001, 2002])], RUN_1_AT).expect("run 1");
    println!("Run 1: EURUSD/390 ids [2001, 2002]");
    manifest.append_all(&[entry("EURUSD", 390, &[2002, 2003])], RUN_2_AT).expect("run 2");
    println!("Run 2: EURUSD/390 ids [2002, 2003]");
    println!("On-disk rows after run 2:");
    dump_rows(tmp.path());
    // merge_entries: build the set of incoming (instrument, mms_id, id) keys,
    // retain() existing rows NOT in that set, then extend with fresh rows.
    println!("  -> 2001 kept from run 1 (not in incoming set)");
    println!("  -> 2002 dropped and re-added: timestamp refreshed, no duplicate");
    println!("  -> row order = surviving old rows first, then this run's rows");
}

// ============================================================
// SECTION 4: Batch Dedup and Empty Entries
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_batch_dedup_and_empty() {
    println!("============================================================");
    println!("SECTION 4: Batch Dedup and Empty Entries");
    println!("============================================================");
    let (tmp, manifest) = fresh_manifest();
    // Same cell twice in ONE call, sharing id 2003.
    let entries = [
        entry("EURUSD", 390, &[2002, 2003]),
        entry("EURUSD", 390, &[2003, 2004]),
        entry("GBPUSD", 410, &[]),
    ];
    println!("append_all: EURUSD/390 [2002,2003] + EURUSD/390 [2003,2004] + GBPUSD/410 []");
    manifest.append_all(&entries, RUN_1_AT).expect("append");
    dump_rows(tmp.path());
    // incoming.insert(...) returns false for the second 2003, so it is
    // pushed to fresh_rows only once.
    println!("  -> 2003 appears once: the HashSet insert doubles as a dedup check");
    println!("  -> GBPUSD/410 has no ids -> contributes no rows at all");
    let map = manifest.read_processed_ids_per_cell().expect("map");
    println!("  -> GBPUSD/410 in map? {}", map.contains_key(&Cell::new("GBPUSD", 410)));
}

// ============================================================
// SECTION 5: Skip Lookup - How Callers Use the Per-Cell Map
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_skip_lookup() {
    println!("============================================================");
    println!("SECTION 5: Skip Lookup - How Callers Use the Per-Cell Map");
    println!("============================================================");
    let (_tmp, manifest) = fresh_manifest();
    manifest.append_all(&[entry("EURUSD", 390, &[2001, 2002]), entry("EURUSD", 391, &[2002])], RUN_1_AT).expect("append");
    let map = manifest.read_processed_ids_per_cell().expect("map");
    let mut cells: Vec<_> = map.iter().collect();
    cells.sort_by_key(|(c, _)| (c.instrument.clone(), c.mms_id));
    for (cell, ids) in cells {
        let mut ids: Vec<_> = ids.iter().copied().collect();
        ids.sort_unstable();
        println!("  {}/{} -> {:?}", cell.instrument, cell.mms_id, ids);
    }
    // processed_at is dropped here: callers only need membership.
    let wanted = [2001, 2002, 2003, 2004];
    let done = map.get(&Cell::new("EURUSD", 390)).cloned().unwrap_or_default();
    let todo: Vec<_> = wanted.iter().filter(|id| !done.contains(id)).collect();
    println!("Source has EURUSD/390 indicators {wanted:?}");
    println!("  -> still to compute: {todo:?}");
    println!("  -> same id 2002 under mms 391 is a separate cell: key is the full Cell");
    let unknown = map.get(&Cell::new("USDJPY", 400));
    println!("  -> unseen cell USDJPY/400: {unknown:?} -> compute everything");
}

// ============================================================
// SECTION 6: Atomic Write - Temp File, Rename, Cleanup
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_atomic_write() {
    println!("============================================================");
    println!("SECTION 6: Atomic Write - Temp File, Rename, Cleanup");
    println!("============================================================");
    // temp_path_for: <root>/processed.parquet.tmp-<pid>-<seq>. Same directory
    // => rename(2) is atomic; pid+seq => concurrent writers never collide.
    println!("Temp name pattern: processed.parquet.tmp-{}-<seq>", std::process::id());
    let (tmp, manifest) = fresh_manifest();
    manifest.append_all(&[entry("EURUSD", 390, &[2001])], RUN_1_AT).expect("ok");
    println!("After success, dir: {:?}", list_dir(tmp.path()));
    // Force rename failure: make processed.parquet a NON-empty directory.
    let (tmp2, manifest2) = fresh_manifest();
    let target = tmp2.path().join("processed.parquet");
    fs::create_dir(&target).expect("dir");
    fs::write(target.join("occupant"), b"").expect("occupant");
    // read_all first: File::open on a directory succeeds on Unix, then
    // parquet decoding fails, so append_all errors BEFORE writing anything.
    let err = manifest2.append_all(&[entry("EURUSD", 390, &[2001])], RUN_1_AT).unwrap_err();
    println!("Target is a directory -> append_all error:");
    println!("  {err:#}");
    println!("After failure, dir: {:?}", list_dir(tmp2.path()));
    println!("  -> readers see either the old file or the new one, never half");
    println!("  -> on write/rename error, remove_temp_file deletes the temp");
    println!("     (NotFound ignored; other removal errors only warn!)");
}

// ============================================================
// SECTION 7: Error Paths - Unopenable, Corrupt, Drifted Schema
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_7_error_paths() {
    println!("============================================================");
    println!("SECTION 7: Error Paths - Unopenable, Corrupt, Drifted Schema");
    println!("============================================================");
    // a) root_dir is a regular file -> open fails with NotADirectory, not NotFound.
    let tmp = TempDir::new().expect("tempdir");
    let not_a_dir = tmp.path().join("root");
    fs::write(&not_a_dir, b"").expect("file");
    let err = ParquetProcessedManifest::new(not_a_dir).read_processed_ids_per_cell().unwrap_err();
    println!("a) root is a file:\n   {err:#}");
    // b) garbage bytes -> opens fine, parquet footer check fails.
    let (tmp, manifest) = fresh_manifest();
    fs::write(tmp.path().join("processed.parquet"), b"not parquet").expect("write");
    let err = manifest.read_processed_ids_per_cell().unwrap_err();
    println!("b) corrupt file:\n   {err:#}");
    // c) mms_id written as Int64 -> parquet_columns returns an error, no panic.
    let (tmp, manifest) = fresh_manifest();
    let batch = RecordBatch::try_from_iter([
        ("instrument", Arc::new(StringArray::from(vec!["EURUSD"])) as ArrayRef),
        ("mms_id", Arc::new(Int64Array::from(vec![390_i64])) as ArrayRef),
        ("indicator_id", Arc::new(Int32Array::from(vec![2001])) as ArrayRef),
        ("processed_at", Arc::new(Int64Array::from(vec![RUN_1_AT])) as ArrayRef),
    ]).expect("batch");
    let mut w = ArrowWriter::try_new(File::create(tmp.path().join("processed.parquet")).expect("f"), batch.schema(), None).expect("w");
    w.write(&batch).expect("write");
    w.close().expect("close");
    let err = manifest.read_processed_ids_per_cell().unwrap_err();
    println!("c) drifted column type:\n   {err:#}");
    // An unreadable ledger also blocks append_all (it reads first), so a bad
    // file stops the run instead of being silently overwritten.
    println!("unix_now_secs() today -> {}", unix_now_secs().expect("clock"));
}
