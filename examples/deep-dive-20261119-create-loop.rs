//! Deep Dive: `bte strategy create_loop` — src/engine/strategy/create_loop.rs
//! Source: Code input — execute / persist_computed_cells / report_run / format_failed_cells
//!
//! create_loop.rs is the batch orchestrator for EVERY (instrument, mms_id) cell:
//!   env -> SourceIndex::load (one bulk DB read) -> cells()
//!   -> read the WHOLE processed manifest once -> compute_cells (parallel, in memory)
//!   -> persist_computed_cells (one write for all cells) -> report_run -> exit verdict.
//!
//! compute_cells itself is covered by deep-dive-20261119-strategy-batch; this script
//! focuses on what create_loop.rs owns: ordering, persisting, reporting, exit status.
//!
//! CLI DETECTED: the real command reads PostgreSQL, so this script never runs it.
//! `mod create_loop` is private, so sections replay its private helpers verbatim and
//! call the public building blocks (BatchSummary, fail_on_total_failure, persistence).

use std::env;
use std::fs;
use std::time::Instant;

use anyhow::Context;
use backtesting_engine::application::cell_output::CellOutput;
use backtesting_engine::application::strategy_batch::{BatchSummary, fail_on_total_failure};
use backtesting_engine::domain::cell::Cell;
use backtesting_engine::infrastructure::strategy_persistence::StrategyPersistence;
use itertools::Itertools;
use tempfile::TempDir;

// ---- Replays of the private create_loop helpers (log macros -> println) ----

fn format_failed_cells(failed_cells: &[Cell]) -> String {
    failed_cells.iter().join(", ")
}

fn persist_computed_cells(p: &StrategyPersistence, computed: Vec<CellOutput>) -> anyhow::Result<()> {
    let cell_count = computed.len();
    p.persist_cells(computed)
        .with_context(|| format!("failed to persist {cell_count} computed cells"))?;
    println!("  [INFO]  Persist done: {cell_count} cells");
    Ok(())
}

fn report_run(summary: &BatchSummary, total_cells: usize) {
    println!(
        "  [INFO]  Done: {} computed, {} skipped (already processed), {} failed.",
        summary.computed, summary.skipped, summary.failed()
    );
    if summary.failed() > 0 {
        println!(
            "  [ERROR] {} of {total_cells} cells failed and will be retried on the next run: {}",
            summary.failed(),
            format_failed_cells(&summary.failed_cells)
        );
    }
}

// ---- Seed data ----

fn summary(computed: usize, skipped: usize, failed_cells: Vec<Cell>) -> BatchSummary {
    BatchSummary { computed, skipped, failed_cells }
}

fn done(instrument: &str, mms_id: i32, ids: &[i32]) -> CellOutput {
    CellOutput { cell: Cell::new(instrument, mms_id), strategies: Vec::new(), computed_indicator_ids: ids.to_vec() }
}

fn fresh_persistence() -> (TempDir, StrategyPersistence) {
    let tmp = TempDir::new().expect("tempdir");
    let persistence = StrategyPersistence::new(tmp.path().to_path_buf());
    (tmp, persistence)
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
        "2" => section_2_format_failed_cells(),
        "3" => section_3_report_and_verdict(),
        "4" => section_4_persist_computed_cells(),
        "5" => section_5_failed_cells_are_retried(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-create-loop <section>");
    println!();
    println!("  CLI DETECTED - section 1 prints the command, never runs it");
    println!();
    println!("  list - Show this list");
    println!("  1    - CLI Entry - create_loop vs create, Step by Step");
    println!("  2    - format_failed_cells - Joining Cells for the Error Line");
    println!("  3    - report_run and the Exit Verdict - Three Batch Shapes");
    println!("  4    - persist_computed_cells - Empty Batch, Real Batch, Broken Root");
    println!("  5    - Failed Cells Are Retried - Manifest After a Partial Run");
}

// ============================================================
// SECTION 1: CLI Entry - create_loop vs create, Step by Step
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_cli_entry() {
    banner("SECTION 1: CLI Entry - create_loop vs create, Step by Step");
    println!("Run this yourself (needs PostgreSQL):");
    println!("  ENVIRONMENT=development cargo run --bin bte -- strategy create_loop");
    println!();
    println!("execute(), in order:");
    println!("  1. resolve_environment()          entrypoint: only place env is read");
    println!("  2. SourceIndex::load(&DbConfig)    ALL cells' data, 4 parallel queries");
    println!("  3. read_processed_ids_per_cell()   whole manifest, read ONCE");
    println!("  4. compute_cells(...)              in memory, nothing written yet");
    println!("  5. persist_computed_cells(...)     ONE write for every computed cell");
    println!("  6. report_run(...)                 counts + failed-cell list");
    println!("  7. fail_on_total_failure(...)      last expression = exit status");
    println!();
    println!("vs create (one cell): fetches per cell, .remove()s its manifest entry,");
    println!("early-returns when nothing is new, and errors on any failure.");
    println!("create_loop tolerates per-cell failures; only a total failure is Err.");
}

// ============================================================
// SECTION 2: format_failed_cells - Joining Cells for the Error Line
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_format_failed_cells() {
    banner("SECTION 2: format_failed_cells - Joining Cells for the Error Line");
    // Itertools::join uses each item's Display impl: "{instrument} (mms_id={id})".
    // That is the same shape compute_cell logs per failure ("{cell}: {e:#}"),
    // so grep on "EURUSD (mms_id=7)" finds both the cause and the summary line.
    let cases: [(&str, Vec<Cell>); 3] = [
        ("none", vec![]),
        ("one", vec![Cell::new("EURUSD", 7)]),
        ("two", vec![Cell::new("EURUSD", 7), Cell::new("USDJPY", 3)]),
    ];
    for (label, cells) in &cases {
        println!("  {label:<4} -> {:?}", format_failed_cells(cells));
    }
    println!();
    println!("  -> empty slice gives \"\", but report_run never prints it:");
    println!("     the error line is guarded by summary.failed() > 0");
}

// ============================================================
// SECTION 3: report_run and the Exit Verdict - Three Batch Shapes
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_report_and_verdict() {
    banner("SECTION 3: report_run and the Exit Verdict - Three Batch Shapes");
    let shapes = [
        ("Clean run", summary(3, 1, vec![])),
        ("Partial failure", summary(2, 1, vec![Cell::new("GBPUSD", 2)])),
        ("Total failure", summary(0, 1, vec![Cell::new("EURUSD", 7), Cell::new("USDJPY", 3)])),
    ];
    for (label, s) in &shapes {
        let total = s.computed + s.skipped + s.failed();
        println!("{label} ({total} cells):");
        report_run(s, total);
        match fail_on_total_failure(s) {
            Ok(()) => println!("  exit: Ok(()) -> status 0"),
            Err(e) => println!("  exit: Err -> {e}"),
        }
        println!();
    }
    println!("  -> report_run runs BEFORE the verdict, so even a total failure");
    println!("     logs which cells failed before the command returns Err");
}

// ============================================================
// SECTION 4: persist_computed_cells - Empty Batch, Real Batch, Broken Root
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_persist_computed_cells() {
    banner("SECTION 4: persist_computed_cells - Empty Batch, Real Batch, Broken Root");
    let (tmp, p) = fresh_persistence();
    let list = |dir: &TempDir| fs::read_dir(dir.path()).expect("dir").count();

    println!("Empty batch (every cell skipped or failed):");
    persist_computed_cells(&p, Vec::new()).expect("empty persist");
    println!("  entries under root: {} (persist_cells returns early)", list(&tmp));

    println!("Real batch (2 cells):");
    let start = Instant::now();
    persist_computed_cells(&p, vec![done("EURUSD", 1, &[11, 12]), done("USDJPY", 1, &[11])]).expect("persist");
    println!("  entries under root: {} ({:?})", list(&tmp), start.elapsed());

    println!("Broken root (a FILE where the parquet root dir should be):");
    let file_root = tmp.path().join("not_a_dir");
    fs::write(&file_root, b"x").expect("write");
    let broken = StrategyPersistence::new(file_root);
    let err = persist_computed_cells(&broken, vec![done("EURUSD", 1, &[11])]).expect_err("must fail");
    for (depth, cause) in err.chain().enumerate().take(3) {
        println!("  {depth}: {cause}");
    }
    println!();
    println!("  -> `?` in execute: report_run and the verdict are SKIPPED on this path,");
    println!("     and the whole batch's compute work is lost (nothing marked processed)");
}

// ============================================================
// SECTION 5: Failed Cells Are Retried - Manifest After a Partial Run
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_failed_cells_are_retried() {
    banner("SECTION 5: Failed Cells Are Retried - Manifest After a Partial Run");
    let (_tmp, p) = fresh_persistence();
    let all_cells = [Cell::new("EURUSD", 1), Cell::new("GBPUSD", 2), Cell::new("USDJPY", 1)];
    println!("Run 1: compute_cells -> EURUSD/1 + USDJPY/1 computed, GBPUSD/2 failed");
    // Only CellOutputs reach persist_computed_cells; a Failed outcome carries just
    // the Cell (for report_run), so nothing about it lands in the manifest.
    persist_computed_cells(&p, vec![done("EURUSD", 1, &[11, 12]), done("USDJPY", 1, &[11, 12])]).expect("persist");
    report_run(&summary(2, 0, vec![Cell::new("GBPUSD", 2)]), all_cells.len());
    println!();

    println!("Run 2: execute reads the manifest once, before compute_cells:");
    let processed = p.read_processed_ids_per_cell().expect("read");
    for cell in &all_cells {
        let ids = processed.get(cell).map(|s| s.iter().copied().sorted().collect_vec());
        let fate = if ids.is_some() { "skip (AlreadyProcessed)" } else { "compute again" };
        println!("  {cell:<20} manifest={ids:?} -> {fate}");
    }
    println!();
    println!("  -> the retry promise in report_run's error line comes from this:");
    println!("     a cell is 'done' only once its ids are in the manifest");
}
