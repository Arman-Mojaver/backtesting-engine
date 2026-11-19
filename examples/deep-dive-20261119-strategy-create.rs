//! Deep Dive: `bte strategy create` — src/engine/strategy/create.rs
//! Source: Code input — execute / create_strategies / read_processed_indicator_ids
//!
//! create.rs is the orchestrator for ONE (instrument, mms_id) cell:
//!   env -> DbConfig + parquet root -> fetch source data (PostgreSQL)
//!   -> ask the manifest which indicators are already done -> diff
//!   -> early return if nothing new -> run_strategies -> persist_cells.
//!
//! CLI DETECTED: the real command reads PostgreSQL, so this script never runs it.
//! `mod create` is private, so the sections replay its steps with the same public
//! building blocks it calls, against a temp parquet root instead of /app/parquet_files.

use std::env;
use std::fs;

use anyhow::Context;
use backtesting_engine::application::cell_output::CellOutput;
use backtesting_engine::config::environment::{Environment, parquet_root_dir};
use backtesting_engine::domain::cell::Cell;
use backtesting_engine::domain::indicator::{
    Indicator, IndicatorParameters, indicator_ids, unprocessed_indicators,
};
use backtesting_engine::infrastructure::strategy_persistence::StrategyPersistence;
use rustc_hash::FxHashSet;
use tempfile::TempDir;

// ---- Seed data: what fetch_source_data would return for EURUSD ----
const INSTRUMENT: &str = "EURUSD";
const MMS_ID: i32 = 390;

fn rsi(id: i32, n: i32) -> Indicator {
    Indicator { id, parameters: IndicatorParameters::Rsi { n, high_threshold: 70, low_threshold: 30 } }
}

fn fetched_indicators() -> Vec<Indicator> {
    vec![rsi(2001, 14), rsi(2002, 21), rsi(2003, 28)]
}

fn fresh_persistence() -> (TempDir, StrategyPersistence) {
    let tmp = TempDir::new().expect("tempdir");
    let persistence = StrategyPersistence::new(tmp.path().to_path_buf());
    (tmp, persistence)
}

/// Same body as the private create::read_processed_indicator_ids.
fn read_processed_indicator_ids(p: &StrategyPersistence, instrument: &str, mms_id: i32) -> anyhow::Result<FxHashSet<i32>> {
    let mut per_cell = p
        .read_processed_ids_per_cell()
        .with_context(|| format!("failed to read processed manifest for {instrument} (mms_id={mms_id})"))?;
    Ok(per_cell.remove(&Cell::new(instrument, mms_id)).unwrap_or_default())
}

/// One pass of create_strategies minus the DB fetch and run_strategies:
/// the persisted CellOutput has no strategies, only the processed ids.
fn simulate_run(p: &StrategyPersistence, indicators: &[Indicator]) -> anyhow::Result<()> {
    let processed = read_processed_indicator_ids(p, INSTRUMENT, MMS_ID)?;
    let unprocessed = unprocessed_indicators(indicators, &processed);
    let ids = indicator_ids(&unprocessed);
    if unprocessed.is_empty() {
        println!("  -> All {} indicators already processed. Nothing to do. (early return)", indicators.len());
        return Ok(());
    }
    println!("  -> running {} unprocessed of {} indicators: {ids:?}", ids.len(), indicators.len());
    p.persist_cells(vec![CellOutput { cell: Cell::new(INSTRUMENT, MMS_ID), strategies: Vec::new(), computed_indicator_ids: ids }])
}

fn sorted(set: &FxHashSet<i32>) -> Vec<i32> {
    let mut v: Vec<i32> = set.iter().copied().collect();
    v.sort_unstable();
    v
}

fn banner(title: &str) {
    println!("============================================================");
    println!("{title}");
    println!("============================================================");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_cli_entry(),
        "2" => section_2_environment(),
        "3" => section_3_first_run(),
        "4" => section_4_cell_isolation(),
        "5" => section_5_resume_and_early_return(),
        "6" => section_6_error_context(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-strategy-create <section>");
    println!();
    println!("  CLI DETECTED - section 1 prints the command, never runs it");
    println!();
    println!("  list - Show this list");
    println!("  1    - CLI Entry - From bte strategy create to create_strategies");
    println!("  2    - Environment - DbConfig and Parquet Root Resolution");
    println!("  3    - First Run - Empty Manifest Means Every Indicator Is New");
    println!("  4    - Cell Isolation - Processed IDs Are Scoped to One Cell");
    println!("  5    - Resume and Early Return - Only New Indicators Run");
    println!("  6    - Error Context - Corrupt Manifest Surfaces the Cell");
}

// ============================================================
// SECTION 1: CLI Entry - From bte strategy create to create_strategies
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_cli_entry() {
    banner("SECTION 1: CLI Entry - From bte strategy create to create_strategies");
    println!("Run this yourself (needs PostgreSQL):");
    println!();
    println!("  ENVIRONMENT=development cargo run --bin bte -- strategy create -m {MMS_ID} -i {INSTRUMENT}");
    println!();
    println!("Dispatch chain:");
    println!("  EngineCommand::Strategy -> StrategyCommand::Create {{ mms_id, instrument }}");
    println!("  -> create::execute        (only times the whole command)");
    println!("  -> create::create_strategies (all the work, see sections 2-6)");
    println!();
    println!("Notes:");
    println!("  -i defaults to EURUSD; -m is required and parsed as i32");
    println!("  execute logs 'Total command time' only on success: `?` returns first");
}

// ============================================================
// SECTION 2: Environment - DbConfig and Parquet Root Resolution
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_environment() {
    banner("SECTION 2: Environment - DbConfig and Parquet Root Resolution");
    // create_strategies is an entrypoint: it is the ONLY place in this flow that
    // resolves Environment. Everything downstream receives DbConfig / PathBuf.
    for env in [Environment::Production, Environment::Development, Environment::Testing] {
        println!("  ENVIRONMENT={:<12} -> parquet root {}", env.as_str(), parquet_root_dir(env).display());
    }
    println!();
    println!("  ENVIRONMENT unset -> defaults to development");
    println!("  ENVIRONMENT=bogus -> resolve_environment() errors, before any DB call");
    println!();
    println!("  StrategyPersistence::new(root) wires two stores under that root:");
    println!("    <root>/strategy/...         partitioned strategy rows");
    println!("    <root>/processed.parquet    the processed-indicator manifest");
}

// ============================================================
// SECTION 3: First Run - Empty Manifest Means Every Indicator Is New
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_first_run() {
    banner("SECTION 3: First Run - Empty Manifest Means Every Indicator Is New");
    let (tmp, p) = fresh_persistence();
    let indicators = fetched_indicators();
    println!("Fetched indicators: {:?}", indicator_ids(&indicators));

    // No processed.parquet yet: the manifest reads as an empty map, not an error.
    let processed = read_processed_indicator_ids(&p, INSTRUMENT, MMS_ID).expect("read");
    println!("processed ids for {INSTRUMENT}/{MMS_ID}: {:?}", sorted(&processed));

    simulate_run(&p, &indicators).expect("run");
    let files: Vec<String> = fs::read_dir(tmp.path()).expect("dir")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned()).collect();
    println!("Files under root after persist: {files:?}");

    let processed = read_processed_indicator_ids(&p, INSTRUMENT, MMS_ID).expect("read");
    println!("processed ids now: {:?}", sorted(&processed));
    println!();
    println!("  -> computed_indicator_ids is written even when 0 strategies came out,");
    println!("     so indicators that produced nothing are not recomputed next time");
}

// ============================================================
// SECTION 4: Cell Isolation - Processed IDs Are Scoped to One Cell
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_cell_isolation() {
    banner("SECTION 4: Cell Isolation - Processed IDs Are Scoped to One Cell");
    let (_tmp, p) = fresh_persistence();
    let done = |inst: &str, mms: i32, ids: &[i32]| CellOutput {
        cell: Cell::new(inst, mms), strategies: Vec::new(), computed_indicator_ids: ids.to_vec(),
    };
    p.persist_cells(vec![done("EURUSD", 390, &[2001, 2002]), done("EURUSD", 391, &[3001]), done("USDJPY", 390, &[4001])])
        .expect("persist");
    println!("Manifest holds 3 cells: EURUSD/390, EURUSD/391, USDJPY/390");
    println!();
    // read_processed_ids_per_cell loads the WHOLE manifest; .remove(&cell) takes one
    // entry by value (no clone) and unwrap_or_default turns 'unseen cell' into {}.
    for (inst, mms) in [("EURUSD", 390), ("EURUSD", 391), ("USDJPY", 390), ("GBPUSD", 390)] {
        let ids = read_processed_indicator_ids(&p, inst, mms).expect("read");
        println!("  read_processed_indicator_ids({inst}, {mms}) -> {:?}", sorted(&ids));
    }
    println!();
    println!("  -> same indicator id on another mms_id/instrument is still unprocessed");
}

// ============================================================
// SECTION 5: Resume and Early Return - Only New Indicators Run
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_resume_and_early_return() {
    banner("SECTION 5: Resume and Early Return - Only New Indicators Run");
    let (_tmp, p) = fresh_persistence();

    println!("Run 1: DB returns 2001, 2002");
    simulate_run(&p, &fetched_indicators()[..2]).expect("run 1");

    println!("Run 2: DB now also returns 2003 (new indicator added)");
    simulate_run(&p, &fetched_indicators()).expect("run 2");

    println!("Run 3: same 3 indicators again");
    simulate_run(&p, &fetched_indicators()).expect("run 3");

    let processed = read_processed_indicator_ids(&p, INSTRUMENT, MMS_ID).expect("read");
    println!();
    println!("Final processed ids: {:?}", sorted(&processed));
    println!("  -> run 3 returns Ok(()) before run_strategies and persist_cells");
    println!("  -> the PostgreSQL fetch still happens every run: the diff needs it");
}

// ============================================================
// SECTION 6: Error Context - Corrupt Manifest Surfaces the Cell
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_error_context() {
    banner("SECTION 6: Error Context - Corrupt Manifest Surfaces the Cell");
    let (tmp, p) = fresh_persistence();
    fs::write(tmp.path().join("processed.parquet"), b"not a parquet file").expect("write");
    println!("Wrote garbage to <root>/processed.parquet");
    println!();

    let err = read_processed_indicator_ids(&p, INSTRUMENT, MMS_ID).expect_err("must fail");
    println!("Error chain (outermost first):");
    for (depth, cause) in err.chain().enumerate().take(4) {
        println!("  {depth}: {cause}");
    }
    println!();
    println!("  -> with_context adds instrument + mms_id on top of the low-level cause");
    println!("  -> run_strategies and persist_cells wrap their errors the same way,");
    println!("     so the top-level error always names the failing cell");
}
