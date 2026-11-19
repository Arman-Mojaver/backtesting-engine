//! Deep Dive: source_data — fetch_source_data and the SourceData bundle
//! Source: Code input — src/application/source_data.rs
//!
//! `bte strategy create` needs four inputs for one cell (instrument + mms_id):
//! every indicator, the instrument's daily points, and the cell's long and
//! short operation points. fetch_source_data opens one PgStore and reads them
//! in sequence, wrapping each query's error with which read failed.
//!
//! No database is needed. Section 2 calls the real function against
//! 127.0.0.1:1, where nothing listens, so it fails at connect. Sections 3-4
//! replay the same four reads against an in-memory store that implements the
//! same repository traits; `replay` mirrors the body of fetch_source_data
//! (the real one hard-wires PgStore, so a fake cannot be passed in).

use std::env;

use anyhow::{Context, bail};
use backtesting_engine::application::source_data::{SourceData, fetch_source_data};
use backtesting_engine::config::environment::DbConfig;
use backtesting_engine::domain::indicator::{
    Indicator, IndicatorParameters, IndicatorRepository, MovingAverageType,
};
use backtesting_engine::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, OperationPoints, ShortOperationPointRepository,
};
use backtesting_engine::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};

// ---- Seed data: one EURUSD cell, mms_id 390, three trading days ----
const DAYS: [i64; 3] = [1_731_888_000, 1_731_974_400, 1_732_060_800]; // 2024-11-18..20

fn indicators() -> Vec<Indicator> {
    let rsi = IndicatorParameters::Rsi { n: 14, high_threshold: 70, low_threshold: 30 };
    let macd = IndicatorParameters::Macd {
        slow_type: MovingAverageType::Ema,
        slow_n: 26,
        fast_type: MovingAverageType::Ema,
        fast_n: 12,
    };
    vec![Indicator { id: 2001, parameters: rsi }, Indicator { id: 2002, parameters: macd }]
}

fn resampled_points() -> Vec<ResampledPointD1> {
    let closes = [1.0541, 1.0597, 1.0484];
    DAYS.iter()
        .zip(closes)
        .zip(1..)
        .map(|((&timestamp, close), id)| ResampledPointD1 {
            id,
            instrument: "EURUSD".into(),
            open: close - 0.002,
            high: close + 0.004,
            low: close - 0.005,
            close,
            volume: 84_000,
            timestamp,
        })
        .collect()
}

fn operation_points(first_id: i32, results: [i32; 3]) -> Vec<OperationPoint> {
    DAYS.iter()
        .zip(results)
        .zip(first_id..)
        .map(|((&timestamp, result), id)| OperationPoint {
            id,
            instrument: "EURUSD".into(),
            money_management_strategy_id: 390,
            timestamp,
            result,
            tp: 60,
            sl: 30,
            risk: 0.01,
        })
        .collect()
}

// ---- In-memory store: the same four ports PgStore implements ----
#[derive(Debug, Default)]
struct FakeStore {
    failing_query: Option<&'static str>,
    empty: bool,
}

impl FakeStore {
    fn answer<T>(&self, query: &'static str, rows: impl FnOnce() -> Vec<T>) -> anyhow::Result<Vec<T>> {
        if self.failing_query == Some(query) {
            bail!("db error: relation \"{query}\" does not exist");
        }
        Ok(if self.empty { Vec::new() } else { rows() })
    }
}

impl IndicatorRepository for FakeStore {
    fn get_all(&mut self) -> anyhow::Result<Vec<Indicator>> {
        self.answer("indicator", indicators)
    }
}

impl ResampledPointD1Repository for FakeStore {
    fn get_all(&mut self) -> anyhow::Result<Vec<ResampledPointD1>> {
        self.answer("resampled_point_d1", resampled_points)
    }
    fn get_by_instrument(&mut self, _: &str) -> anyhow::Result<Vec<ResampledPointD1>> {
        self.answer("resampled_point_d1", resampled_points)
    }
}

impl LongOperationPointRepository for FakeStore {
    fn get_all(&mut self) -> anyhow::Result<Vec<OperationPoint>> {
        self.answer("long_operation_point", || operation_points(800, [60, -30, 60]))
    }
    fn get_by_mms_id_and_instrument(&mut self, _: i32, _: &str) -> anyhow::Result<Vec<OperationPoint>> {
        LongOperationPointRepository::get_all(self)
    }
}

impl ShortOperationPointRepository for FakeStore {
    fn get_all(&mut self) -> anyhow::Result<Vec<OperationPoint>> {
        self.answer("short_operation_point", || operation_points(900, [-30, 60, -30]))
    }
    fn get_by_mms_id_and_instrument(&mut self, _: i32, _: &str) -> anyhow::Result<Vec<OperationPoint>> {
        ShortOperationPointRepository::get_all(self)
    }
}

/// Mirror of fetch_source_data's body after `PgStore::connect`, same contexts.
fn replay(store: &mut FakeStore, instrument: &str, mms_id: i32) -> anyhow::Result<SourceData> {
    let indicators = IndicatorRepository::get_all(store).context("failed to fetch indicators")?;
    let resampled_points = ResampledPointD1Repository::get_by_instrument(store, instrument)
        .with_context(|| format!("failed to fetch resampled points for {instrument}"))?;
    let long_operation_points =
        LongOperationPointRepository::get_by_mms_id_and_instrument(store, mms_id, instrument)
            .with_context(|| {
                format!("failed to fetch long operation points for {instrument} (mms_id={mms_id})")
            })?;
    let short_operation_points =
        ShortOperationPointRepository::get_by_mms_id_and_instrument(store, mms_id, instrument)
            .with_context(|| {
                format!("failed to fetch short operation points for {instrument} (mms_id={mms_id})")
            })?;
    Ok(SourceData { indicators, resampled_points, long_operation_points, short_operation_points })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map_or("list", String::as_str) {
        "1" => section_1_source_data_shape(),
        "2" => section_2_connect_failure(),
        "3" => section_3_which_read_failed(),
        "4" => section_4_empty_results(),
        "5" => section_5_create_vs_create_loop(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-source-data <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - SourceData Shape: four vectors for one cell");
    println!("  2    - Connect Failure: the real function stops before any query");
    println!("  3    - Which Read Failed: one context message per query");
    println!("  4    - Empty Results: zero rows is Ok, not an error");
    println!("  5    - create vs create_loop: one cell sequential vs all cells parallel");
}

// ============================================================
// SECTION 1: SourceData Shape - four vectors for one cell
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_source_data_shape() {
    println!("============================================================");
    println!("SECTION 1: SourceData Shape - four vectors for one cell");
    println!("============================================================");
    let Ok(data) = replay(&mut FakeStore::default(), "EURUSD", 390) else { return };
    println!("indicators             : {} (ALL indicators, not filtered by cell)", data.indicators.len());
    for i in &data.indicators {
        println!("  id {} -> {:?}", i.id, i.parameters);
    }
    println!("resampled_points       : {} daily points for EURUSD", data.resampled_points.len());
    if let Some(p) = data.resampled_points.first() {
        println!("  first: ts {} close {:.4}", p.timestamp, p.close);
    }
    println!("long_operation_points  : {} (EURUSD, mms_id 390)", data.long_operation_points.len());
    println!("short_operation_points : {} (EURUSD, mms_id 390)", data.short_operation_points.len());
    println!();
    println!("  -> SourceData is a plain pub-field bundle: create.rs destructures it into");
    println!("     run_strategies(&unprocessed, &resampled, &long, &short, mms_id)");
    println!("  -> indicators are filtered AFTER fetch, against the processed manifest");
}

// ============================================================
// SECTION 2: Connect Failure - the real function stops before any query
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_connect_failure() {
    println!("============================================================");
    println!("SECTION 2: Connect Failure - the real function stops before any query");
    println!("============================================================");
    let config = DbConfig {
        host: "127.0.0.1".into(),
        port: 1,
        user: "engine_role".into(),
        password: "s3cret-pw".into(),
        database: "db-development".into(),
    };
    let Err(err) = fetch_source_data(&config, "EURUSD", 390) else { return };
    let chain: Vec<String> = err.chain().map(ToString::to_string).collect();
    println!("error chain depth: {}", chain.len());
    if let Some(top) = chain.first().and_then(|c| c.lines().next()) {
        println!("[0] {top}  (+ diagnostics lines)");
    }
    if let Some(cause) = chain.last() {
        println!("[{}] {cause}", chain.len().saturating_sub(1));
    }
    println!();
    println!("  -> `PgStore::connect(db_config)?` is propagated as-is: no extra context");
    println!("     here, because connect already says which database it tried");
    println!("  -> no \"failed to fetch ...\" layer: none of the four reads ran");
    println!("  -> the debug! line logs db_config.database only, never the password");
}

// ============================================================
// SECTION 3: Which Read Failed - one context message per query
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_which_read_failed() {
    println!("============================================================");
    println!("SECTION 3: Which Read Failed - one context message per query");
    println!("============================================================");
    for query in ["indicator", "resampled_point_d1", "long_operation_point", "short_operation_point"] {
        let mut store = FakeStore { failing_query: Some(query), empty: false };
        let Err(err) = replay(&mut store, "EURUSD", 390) else { continue };
        println!("{query} fails:");
        println!("  {err:#}");
    }
    println!();
    println!("  -> `?` after each read: the first failure wins, later reads never run");
    println!("  -> indicators use .context (static text); the other three use");
    println!("     .with_context(|| format!(..)) so the String is built only on error");
    println!("  -> long and short messages carry instrument + mms_id: the cell id");
}

// ============================================================
// SECTION 4: Empty Results - zero rows is Ok, not an error
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_empty_results() {
    println!("============================================================");
    println!("SECTION 4: Empty Results - zero rows is Ok, not an error");
    println!("============================================================");
    let mut store = FakeStore { failing_query: None, empty: true };
    let result = replay(&mut store, "XAUUSD", 999);
    println!("fetch for unknown cell XAUUSD/999 -> is_ok: {}", result.is_ok());
    let Ok(data) = result else { return };
    println!(
        "lens: indicators {} resampled {} long {} short {}",
        data.indicators.len(),
        data.resampled_points.len(),
        data.long_operation_points.len(),
        data.short_operation_points.len()
    );
    let ops = OperationPoints {
        long_operation_points: &data.long_operation_points,
        short_operation_points: &data.short_operation_points,
    };
    println!("OperationPoints::start_end_timestamps -> {:?}", ops.start_end_timestamps());
    println!();
    println!("  -> fetch_source_data does not validate: a typo'd instrument or mms_id");
    println!("     returns four empty Vecs, and create.rs carries on");
    println!("  -> no indicators: create.rs logs \"Nothing to do\" and returns Ok");
    println!("  -> otherwise run_strategies' validate_all rejects the empty points");
    println!("     (\"input validation failed\"), far from the actual typo");
}

// ============================================================
// SECTION 5: create vs create_loop - one cell sequential vs all cells parallel
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_create_vs_create_loop() {
    println!("============================================================");
    println!("SECTION 5: create vs create_loop - one cell sequential vs all cells parallel");
    println!("============================================================");
    println!("strategy create  (src/engine/strategy/create.rs -> source_data.rs)");
    println!("  1 PgStore, 4 reads in sequence, WHERE-filtered to one cell");
    println!("  get_by_instrument / get_by_mms_id_and_instrument");
    println!();
    println!("strategy create_loop  (src/application/strategy_batch.rs)");
    println!("  4 PgStores, nested rayon::join, get_all + index_by_* for every cell");
    println!();
    println!("  -> one cell: the filtered queries are small, so sequential reads on");
    println!("     one connection are simpler and fast enough");
    println!("  -> all cells: tables are read whole, so parallel connections pay off");
    println!();
    println!("Try it yourself (needs the dev DB up; this script runs nothing):");
    println!("  RUST_LOG=debug bte strategy create -m 390 -i EURUSD");
    println!("Expect, in order:");
    println!("  Fetching source data for EURUSD (mms_id=390) ...");
    println!("  connecting to db-development");
    println!("  query fetched and mapped (indicator get_all): N rows in ...");
    println!("  Source data fetched in ...");
}
