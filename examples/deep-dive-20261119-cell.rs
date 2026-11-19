//! Deep Dive: Cell — the (instrument, money-management strategy) key
//! Source: Code input — src/domain/cell.rs
//!
//! A Cell names one unit of strategy work: every operation point, processed
//! manifest row and Parquet partition belongs to exactly one Cell. Its derives
//! (Hash/Eq for map keys, Ord for stable sorting, Display for logs) are what
//! the rest of the engine leans on.

use std::collections::BTreeSet;
use std::env;
use std::path::PathBuf;

use backtesting_engine::domain::cell::Cell;
use itertools::Itertools;
use rustc_hash::{FxHashMap, FxHashSet};

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_construction(),
        "2" => section_2_display(),
        "3" => section_3_ordering(),
        "4" => section_4_hash_map_key(),
        "5" => section_5_partition_path(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-cell <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Construction: Cell::new and impl Into<String>");
    println!("  2    - Display: log formatting and failed-cell reports");
    println!("  3    - Ordering: derived Ord sorts by instrument, then mms_id");
    println!("  4    - Map Key: Hash + Eq for per-cell grouping and manifests");
    println!("  5    - Partition Path: how a Cell becomes a Parquet directory");
}

// ============================================================
// SECTION 1: Construction - Cell::new and impl Into<String>
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_construction() {
    println!("============================================================");
    println!("SECTION 1: Construction - Cell::new and impl Into<String>");
    println!("============================================================");

    // `impl Into<String>` lets callers pass either a literal (tests) or an
    // owned String they already have (DB rows), without an extra clone.
    let from_literal = Cell::new("EURUSD", 390);
    let owned = String::from("USDJPY");
    let from_owned = Cell::new(owned, 400); // `owned` is moved, not copied
    println!("from &str  : {from_literal:?}");
    println!("from String: {from_owned:?}");

    // Fields are plain pub — no invariants are enforced.
    let literal = Cell { instrument: "GBPUSD".into(), mms_id: 1 };
    println!("struct lit : {literal:?}");

    // Edge case: nothing rejects an empty instrument or a negative id.
    let odd = Cell::new("", -1);
    println!("edge case  : {odd:?}  (accepted — validation lives upstream)");

    // Clone is a deep copy of the String; equality is field-wise.
    let copy = from_literal.clone();
    println!("clone == original? {}", copy == from_literal);
}

// ============================================================
// SECTION 2: Display - Log Formatting and Failed-Cell Reports
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_display() {
    println!("============================================================");
    println!("SECTION 2: Display - Log Formatting and Failed-Cell Reports");
    println!("============================================================");

    let cell = Cell::new("EURUSD", 7);
    println!("Display: {cell}");
    println!("Debug  : {cell:?}");
    println!();

    // Mirrors the private `format_failed_cells` in
    // src/engine/strategy/create_loop.rs: Display + itertools::join.
    let failed = [Cell::new("EURUSD", 7), Cell::new("USDJPY", 3)];
    println!("failed cells: {}", failed.iter().join(", "));
    println!();
    println!("  -> Display is what operators see in error logs;");
    println!("     Debug stays for developer dumps.");
}

// ============================================================
// SECTION 3: Ordering - Derived Ord Sorts by Instrument, then mms_id
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_ordering() {
    println!("============================================================");
    println!("SECTION 3: Ordering - Derived Ord Sorts by Instrument, then mms_id");
    println!("============================================================");

    // Derived Ord compares fields in declaration order: instrument first.
    let mut cells = vec![
        Cell::new("GBPUSD", 1),
        Cell::new("EURUSD", 10),
        Cell::new("EURUSD", 2),
        Cell::new("USDJPY", 0),
    ];
    println!("before: {}", cells.iter().join(" | "));
    cells.sort();
    println!("after : {}", cells.iter().join(" | "));
    println!();

    // Branch: instrument decides before mms_id even when mms_id is larger.
    let a = Cell::new("AUDUSD", 999);
    let b = Cell::new("EURUSD", 1);
    println!("{a} < {b}? {}", a < b);

    // String ordering is byte-wise: uppercase sorts before lowercase.
    let upper = Cell::new("EURUSD", 1);
    let lower = Cell::new("eurusd", 1);
    println!("{upper} < {lower}? {} (byte-wise, case-sensitive)", upper < lower);

    // BTreeSet uses Ord too: duplicates collapse, iteration is sorted.
    let set: BTreeSet<Cell> = cells.into_iter().chain([Cell::new("EURUSD", 2)]).collect();
    println!("BTreeSet ({} unique): {}", set.len(), set.iter().join(" | "));
}

// ============================================================
// SECTION 4: Map Key - Hash + Eq for Per-Cell Grouping and Manifests
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_hash_map_key() {
    println!("============================================================");
    println!("SECTION 4: Map Key - Hash + Eq for Per-Cell Grouping and Manifests");
    println!("============================================================");

    // Seed: (instrument, mms_id, strategy_id) rows, like the processed manifest.
    let manifest_rows = [
        ("EURUSD", 390, 11),
        ("EURUSD", 390, 12),
        ("EURUSD", 391, 11),
        ("USDJPY", 390, 11),
        ("EURUSD", 390, 12), // duplicate id — the set absorbs it
    ];

    // Same shape as ProcessedManifest::read_processed_ids_per_cell.
    let mut processed: FxHashMap<Cell, FxHashSet<i32>> = FxHashMap::default();
    for (instrument, mms_id, id) in manifest_rows {
        processed.entry(Cell::new(instrument, mms_id)).or_default().insert(id);
    }

    // Sort for deterministic output — HashMap iteration order is not stable.
    for (cell, ids) in processed.iter().sorted_by(|a, b| a.0.cmp(b.0)) {
        println!("{cell} -> ids {:?}", ids.iter().sorted().collect_vec());
    }
    println!();

    // Lookup by a freshly built key: equality is by value, not identity.
    let hit = processed.get(&Cell::new("EURUSD", 391));
    let miss = processed.get(&Cell::new("EURUSD", 999));
    println!("get(EURUSD, 391) -> {hit:?}");
    println!("get(EURUSD, 999) -> {miss:?}  (unseen cell => nothing processed)");

    // Both fields participate: same instrument, different mms_id => distinct key.
    println!("distinct cells: {}", processed.len());
}

// ============================================================
// SECTION 5: Partition Path - How a Cell Becomes a Parquet Directory
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_partition_path() {
    println!("============================================================");
    println!("SECTION 5: Partition Path - How a Cell Becomes a Parquet Directory");
    println!("============================================================");

    // Mirrors the private ParquetStrategyRepository::partition_path in
    // src/infrastructure/strategy_adapter.rs (Hive-style key=value dirs).
    // Nothing is written — this only builds the PathBuf.
    let base_dir = PathBuf::from("data/strategies");
    for cell in [Cell::new("EURUSD", 390), Cell::new("USDJPY", 400)] {
        let path = base_dir
            .join(format!("instrument={}", cell.instrument))
            .join(format!("money_management_strategy_id={}", cell.mms_id))
            .join("strategies.parquet");
        println!("{cell}");
        println!("  -> {}", path.display());
    }
    println!();
    println!("  -> One Cell == one partition file; write_rows rewrites it in place.");
    println!("  -> The field is `mms_id` in code but the full name on disk,");
    println!("     so Parquet readers see self-describing partition columns.");
}
