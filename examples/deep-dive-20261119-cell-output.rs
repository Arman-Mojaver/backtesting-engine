//! Deep Dive: CellOutput — the hand-off between compute and persist
//! Source: Code input — src/application/cell_output.rs
//!
//! CellOutput is a plain carrier with no methods. It matters because of how
//! it is produced (strategy_batch::compute_cell, strategy::create) and
//! consumed (StrategyPersistence::persist_cells). The two Vecs it carries
//! answer different questions:
//!   strategies             -> what goes into the cell's Parquet partition
//!   computed_indicator_ids -> what the manifest marks as "done", so reruns skip it

use std::env;
use std::path::Path;

use backtesting_engine::application::cell_output::CellOutput;
use backtesting_engine::domain::cell::Cell;
use backtesting_engine::domain::indicator::{
    Indicator, IndicatorParameters, indicator_ids, unprocessed_indicators,
};
use backtesting_engine::domain::strategy::Strategy;
use backtesting_engine::infrastructure::strategy_adapter::ParquetStrategyRepository;
use backtesting_engine::infrastructure::strategy_persistence::StrategyPersistence;
use rustc_hash::FxHashSet;

// ---- Seed data ----
fn rsi(id: i32, n: i32) -> Indicator {
    Indicator {
        id,
        parameters: IndicatorParameters::Rsi {
            n,
            high_threshold: 70,
            low_threshold: 30,
        },
    }
}

fn all_indicators() -> Vec<Indicator> {
    vec![rsi(101, 14), rsi(102, 21), rsi(103, 28)]
}

fn strategy(cell: &Cell, indicator_id: i32, annual_roi: f64) -> Strategy {
    Strategy {
        instrument: cell.instrument.clone(),
        indicator_id,
        annual_roi,
        max_draw_down: 0.12,
        annual_operation_count: 24.0,
        long_operation_point_ids: vec![1, 3],
        short_operation_point_ids: vec![2],
        money_management_strategy_id: cell.mms_id,
    }
}

// Mirrors strategy_batch::compute_cell, whose CellOutcome enum is private.
// Indicator 102 is pretended to be dropped by the annual-ops filter.
fn compute(cell: &Cell, processed: &FxHashSet<i32>) -> Option<CellOutput> {
    let unprocessed = unprocessed_indicators(&all_indicators(), processed);
    if unprocessed.is_empty() {
        return None; // CellOutcome::AlreadyProcessed
    }
    let strategies = unprocessed
        .iter()
        .filter(|indicator| indicator.id != 102)
        .map(|indicator| strategy(cell, indicator.id, 0.08))
        .collect();
    Some(CellOutput {
        cell: cell.clone(),
        strategies,
        computed_indicator_ids: indicator_ids(&unprocessed),
    })
}

fn summarize(output: &CellOutput) -> String {
    let strategy_ids: Vec<i32> = output.strategies.iter().map(|s| s.indicator_id).collect();
    format!(
        "{} strategies={strategy_ids:?} computed_ids={:?}",
        output.cell, output.computed_indicator_ids
    )
}

fn partition_file(root: &Path, cell: &Cell) -> std::path::PathBuf {
    root.join("strategy")
        .join(format!("instrument={}", cell.instrument))
        .join(format!("money_management_strategy_id={}", cell.mms_id))
        .join("strategies.parquet")
}

fn banner(title: &str) {
    println!("============================================================");
    println!("{title}");
    println!("============================================================");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_shape(),
        "2" => section_2_two_lists(),
        "3" => section_3_producer_branches(),
        "4" => section_4_persist(),
        "5" => section_5_rerun(),
        "6" => section_6_no_strategies(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-cell-output <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Shape: three pub fields, Debug only, moved not cloned");
    println!("  2    - Two Lists: strategies vs computed_indicator_ids");
    println!("  3    - Producer Branches: Computed / AlreadyProcessed / Failed");
    println!("  4    - Persist: partitions, global strategy ids, manifest");
    println!("  5    - Rerun: the manifest turns the next run into a skip");
    println!("  6    - No Strategies: manifest advances, partition untouched");
}

// ============================================================
// SECTION 1: Shape - Three Pub Fields, Debug Only, Moved Not Cloned
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_shape() {
    banner("SECTION 1: Shape - Three Pub Fields, Debug Only, Moved Not Cloned");
    let cell = Cell::new("EURUSD", 7);
    let output = CellOutput {
        cell: cell.clone(),
        strategies: vec![strategy(&cell, 101, 0.08)],
        computed_indicator_ids: vec![101],
    };
    println!("cell                   : {}", output.cell);
    println!("strategies.len()       : {}", output.strategies.len());
    println!(
        "computed_indicator_ids : {:?}",
        output.computed_indicator_ids
    );
    println!();
    // Only #[derive(Debug)]: no Clone, no PartialEq. persist_cells takes
    // Vec<CellOutput> by value and moves each Strategy into a StrategyRow,
    // so a full strategy set is never copied between compute and write.
    println!("Debug (truncated): {:.90}...", format!("{output:?}"));
    println!();
    println!("  -> No constructor, no invariants: callers build it with a literal");
    println!("  -> Ownership flows compute -> Vec<CellOutput> -> persist_cells");
}

// ============================================================
// SECTION 2: Two Lists - strategies vs computed_indicator_ids
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_two_lists() {
    banner("SECTION 2: Two Lists - strategies vs computed_indicator_ids");
    let cell = Cell::new("EURUSD", 7);
    let output = compute(&cell, &FxHashSet::default()).expect("fresh cell computes");
    println!("{}", summarize(&output));

    let with_strategy: FxHashSet<i32> = output.strategies.iter().map(|s| s.indicator_id).collect();
    let without: Vec<i32> = output
        .computed_indicator_ids
        .iter()
        .copied()
        .filter(|id| !with_strategy.contains(id))
        .collect();
    println!("computed but produced no strategy: {without:?}");
    println!();
    // Deriving "done" from strategies would forget indicator 102, and every
    // rerun would recompute it only to filter it out again.
    println!("  -> computed_indicator_ids = everything that was ATTEMPTED");
    println!("  -> strategies             = only what SURVIVED the filters");
    println!("  -> So computed_indicator_ids ⊇ strategies' indicator_ids");
}

// ============================================================
// SECTION 3: Producer Branches - Computed / AlreadyProcessed / Failed
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_producer_branches() {
    banner("SECTION 3: Producer Branches - Computed / AlreadyProcessed / Failed");
    let cell = Cell::new("USDJPY", 3);

    let partial: FxHashSet<i32> = [101].into_iter().collect();
    let output = compute(&cell, &partial).expect("102, 103 still pending");
    println!(
        "processed={{101}}         -> Computed: {}",
        summarize(&output)
    );

    let everything: FxHashSet<i32> = [101, 102, 103].into_iter().collect();
    println!(
        "processed={{101,102,103}} -> {:?} (AlreadyProcessed)",
        compute(&cell, &everything).map(|o| summarize(&o))
    );
    println!();
    // A run_strategies error yields CellOutcome::Failed(cell): no CellOutput
    // exists, so nothing reaches the manifest and the cell retries next run.
    println!("run_strategies Err       -> Failed(cell): no CellOutput built");
    println!();
    println!("  -> Only the Computed branch produces a CellOutput");
    println!("  -> Partial manifests compute just the missing indicators");
}

// ============================================================
// SECTION 4: Persist - Partitions, Global Strategy Ids, Manifest
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_persist() {
    banner("SECTION 4: Persist - Partitions, Global Strategy Ids, Manifest");
    let dir = tempfile::tempdir().expect("tempdir");
    let persistence = StrategyPersistence::new(dir.path().to_path_buf());
    let eur = Cell::new("EURUSD", 7);
    let jpy = Cell::new("USDJPY", 3);
    let outputs: Vec<CellOutput> = [&eur, &jpy]
        .into_iter()
        .filter_map(|cell| compute(cell, &FxHashSet::default()))
        .collect();
    for output in &outputs {
        println!("in : {}", summarize(output));
    }

    persistence.persist_cells(outputs).expect("persist");

    for cell in [&eur, &jpy] {
        println!(
            "partition {cell}: exists={}",
            partition_file(dir.path(), cell).exists()
        );
    }
    let repo = ParquetStrategyRepository::new(dir.path().join("strategy"));
    println!(
        "max strategy_id across tree: {}",
        repo.max_strategy_id().expect("scan")
    );
    let manifest = persistence.read_processed_ids_per_cell().expect("manifest");
    for cell in [&eur, &jpy] {
        let mut ids: Vec<i32> = manifest[cell].iter().copied().collect();
        ids.sort_unstable();
        println!("manifest {cell}: {ids:?}");
    }
    println!();
    println!("  -> 4 strategies got ids 1..=4, numbered across cells, not per cell");
    println!("  -> Manifest includes 102 even though no strategy row has it");
    println!("  -> Partitions are written first, the manifest last");
}

// ============================================================
// SECTION 5: Rerun - The Manifest Turns the Next Run into a Skip
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_rerun() {
    banner("SECTION 5: Rerun - The Manifest Turns the Next Run into a Skip");
    let dir = tempfile::tempdir().expect("tempdir");
    let persistence = StrategyPersistence::new(dir.path().to_path_buf());
    let cell = Cell::new("EURUSD", 7);

    let first = compute(&cell, &FxHashSet::default()).expect("first run computes");
    persistence.persist_cells(vec![first]).expect("persist");

    let manifest = persistence.read_processed_ids_per_cell().expect("manifest");
    let second = compute(&cell, &manifest[&cell]);
    println!("second run CellOutput: {:?}", second.map(|o| summarize(&o)));

    // An empty batch returns before even scanning for max_strategy_id.
    persistence.persist_cells(Vec::new()).expect("empty batch");
    println!("persist_cells(vec![]) -> Ok, no files touched");
    println!();
    println!("  -> computed_indicator_ids from run 1 feed processed ids of run 2");
    println!("  -> Fully-processed cells never become a CellOutput at all");
}

// ============================================================
// SECTION 6: No Strategies - Manifest Advances, Partition Untouched
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_no_strategies() {
    banner("SECTION 6: No Strategies - Manifest Advances, Partition Untouched");
    let dir = tempfile::tempdir().expect("tempdir");
    let persistence = StrategyPersistence::new(dir.path().to_path_buf());
    let cell = Cell::new("GBPUSD", 2);
    // Every indicator was filtered out: valid, and common for sparse cells.
    let output = CellOutput {
        cell: cell.clone(),
        strategies: Vec::new(),
        computed_indicator_ids: vec![101, 102, 103],
    };
    println!("in : {}", summarize(&output));
    persistence.persist_cells(vec![output]).expect("persist");

    println!(
        "partition exists: {}",
        partition_file(dir.path(), &cell).exists()
    );
    let manifest = persistence.read_processed_ids_per_cell().expect("manifest");
    println!("manifest has {} ids for {cell}", manifest[&cell].len());
    println!();
    println!("  -> write_partition skips the file when there are 0 fresh rows");
    println!("  -> The manifest still records the attempt, so no recompute");
}
