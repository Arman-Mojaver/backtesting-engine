//! Deep Dive: ResampledPointD1 + ResampledPointD1Repository
//! Source: Code input — src/domain/resampled_point.rs
//!
//! A ResampledPointD1 is one daily OHLCV point for one instrument. The domain
//! defines the repository *trait*; PgStore (src/infrastructure/pg_adapter.rs)
//! implements it. Here an in-memory repository stands in for Postgres, so the
//! script touches no database.

use std::env;

use anyhow::bail;
use backtesting_engine::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};
use itertools::Itertools;

// ---- Seed data: rows as `ORDER BY timestamp` returns them (instruments interleaved) ----
fn point(id: i32, instrument: &str, timestamp: i64, close: f64) -> ResampledPointD1 {
    ResampledPointD1 {
        id,
        instrument: instrument.into(),
        open: close - 0.002,
        high: close + 0.004,
        low: close - 0.005,
        close,
        volume: 41_250,
        timestamp,
    }
}

fn seed_points() -> Vec<ResampledPointD1> {
    vec![
        point(101, "EURUSD", 1_704_153_600, 1.0941), // 2024-01-02
        point(201, "USDJPY", 1_704_153_600, 141.98),
        point(102, "EURUSD", 1_704_240_000, 1.0924), // 2024-01-03
        point(202, "USDJPY", 1_704_240_000, 143.29),
        point(103, "EURUSD", 1_704_326_400, 1.0945), // 2024-01-04
    ]
}

// ---- In-memory stand-in for PgStore ----
#[derive(Debug)]
struct InMemoryStore {
    rows: Vec<ResampledPointD1>,
    fail: bool,
}

impl ResampledPointD1Repository for InMemoryStore {
    fn get_all(&mut self) -> anyhow::Result<Vec<ResampledPointD1>> {
        if self.fail {
            bail!("connection reset by peer");
        }
        Ok(self.rows.clone())
    }

    fn get_by_instrument(&mut self, instrument: &str) -> anyhow::Result<Vec<ResampledPointD1>> {
        let all = self.get_all()?;
        Ok(all.into_iter().filter(|p| p.instrument == instrument).collect())
    }
    // index_by_instrument is NOT implemented here — the trait's default is used.
}

const fn store(rows: Vec<ResampledPointD1>) -> InMemoryStore {
    InMemoryStore { rows, fail: false }
}

fn timestamps(points: &[ResampledPointD1]) -> String {
    points.iter().map(|p| p.timestamp).join(", ")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map_or("list", String::as_str) {
        "1" => section_1_the_point(),
        "2" => section_2_single_instrument(),
        "3" => section_3_index_by_instrument(),
        "4" => section_4_order_is_inherited(),
        "5" => section_5_empty_and_missing(),
        "6" => section_6_error_propagation(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-resampled-point <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - The Point: one daily OHLCV row and its traced id");
    println!("  2    - Single Instrument: get_by_instrument (the bte single-cell path)");
    println!("  3    - Index By Instrument: the trait's default grouping method");
    println!("  4    - Order Is Inherited: grouping never sorts");
    println!("  5    - Empty and Missing: no rows, unknown instrument");
    println!("  6    - Error Propagation: get_all fails, ? bubbles it up");
}

// ============================================================
// SECTION 1: The Point - One Daily OHLCV Row and Its Traced id
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_the_point() {
    println!("============================================================");
    println!("SECTION 1: The Point - One Daily OHLCV Row and Its Traced id");
    println!("============================================================");

    let p = point(101, "EURUSD", 1_704_153_600, 1.0941);
    println!("{p:#?}");
    println!();
    // `id` is never read by the engine; it lets you find the source DB row.
    println!("  -> id={} traces back to resampled_point_d1.id", p.id);
    println!("  -> timestamp is Unix seconds at the start of the day (D1)");
    // Plain pub fields: nothing enforces low <= open/close <= high.
    println!("  -> no invariants: low <= close <= high is {}", p.low <= p.close && p.close <= p.high);
}

// ============================================================
// SECTION 2: Single Instrument - get_by_instrument
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_single_instrument() {
    println!("============================================================");
    println!("SECTION 2: Single Instrument - get_by_instrument");
    println!("============================================================");

    // Used by application::source_data::fetch_source_data (one Cell at a time).
    // PgStore pushes the filter into SQL: WHERE instrument = $1 ORDER BY timestamp.
    let mut repo = store(seed_points());
    let Ok(eurusd) = repo.get_by_instrument("EURUSD") else {
        println!("unexpected error");
        return;
    };
    println!("EURUSD -> {} points", eurusd.len());
    for p in &eurusd {
        println!("  id={} ts={} close={}", p.id, p.timestamp, p.close);
    }
    println!();
    println!("  -> One query per instrument: fine for one cell, slow for a batch.");
    println!("     The batch path uses index_by_instrument instead (section 3).");
}

// ============================================================
// SECTION 3: Index By Instrument - The Trait's Default Grouping Method
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_index_by_instrument() {
    println!("============================================================");
    println!("SECTION 3: Index By Instrument - The Trait's Default Grouping Method");
    println!("============================================================");

    // Used by application::strategy_batch::SourceIndex::load: ONE get_all query,
    // then the private group_by_instrument splits rows into per-instrument Vecs.
    let mut repo = store(seed_points());
    println!("get_all order : {}", repo.rows.iter().map(|p| p.id).join(", "));
    let Ok(index) = repo.index_by_instrument() else {
        println!("unexpected error");
        return;
    };
    // FxHashMap iteration order is unstable — sort keys for deterministic output.
    for (instrument, points) in index.iter().sorted_by(|a, b| a.0.cmp(b.0)) {
        println!("{instrument}: ids {}", points.iter().map(|p| p.id).join(", "));
        println!("        ts  {}", timestamps(points));
    }
    println!();
    println!("  -> Each instrument's Vec keeps the relative order of get_all.");
    println!("  -> Only the first point of an instrument clones the key String;");
    println!("     later points hit get_mut and just push (point is moved, not cloned).");
}

// ============================================================
// SECTION 4: Order Is Inherited - Grouping Never Sorts
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_order_is_inherited() {
    println!("============================================================");
    println!("SECTION 4: Order Is Inherited - Grouping Never Sorts");
    println!("============================================================");

    // Indicator math assumes chronological order. That guarantee lives in the
    // adapter's `ORDER BY timestamp`, not in group_by_instrument.
    let mut shuffled = seed_points();
    shuffled.reverse();
    let mut repo = store(shuffled);
    let Ok(index) = repo.index_by_instrument() else {
        println!("unexpected error");
        return;
    };
    let Some(eurusd) = index.get("EURUSD") else {
        println!("EURUSD missing");
        return;
    };
    println!("unsorted get_all -> EURUSD ts: {}", timestamps(eurusd));
    let sorted = eurusd.iter().tuple_windows().all(|(a, b)| a.timestamp <= b.timestamp);
    println!("chronological? {sorted}");
    println!();
    println!("  -> A repository that forgets ORDER BY silently feeds indicators");
    println!("     reversed data. Any new adapter must sort before returning.");
}

// ============================================================
// SECTION 5: Empty and Missing - No Rows, Unknown Instrument
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_empty_and_missing() {
    println!("============================================================");
    println!("SECTION 5: Empty and Missing - No Rows, Unknown Instrument");
    println!("============================================================");

    let mut empty = store(Vec::new());
    let empty_index = empty.index_by_instrument().map(|i| i.len());
    println!("empty table -> index_by_instrument: {empty_index:?} instruments");

    // A missing instrument is not an error: the map simply has no key.
    let mut repo = store(seed_points());
    let Ok(index) = repo.index_by_instrument() else {
        println!("unexpected error");
        return;
    };
    println!("index.get(\"GBPUSD\") -> {:?}", index.get("GBPUSD").map(Vec::len));
    let single = repo.get_by_instrument("GBPUSD").map(|v| v.len());
    println!("get_by_instrument(\"GBPUSD\") -> {single:?} points");
    println!();
    println!("  -> Neither path errors on an unknown instrument; callers must decide");
    println!("     whether zero points is fatal. Never use index[..] on untrusted keys.");
}

// ============================================================
// SECTION 6: Error Propagation - get_all Fails, ? Bubbles It Up
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_error_propagation() {
    println!("============================================================");
    println!("SECTION 6: Error Propagation - get_all Fails, ? Bubbles It Up");
    println!("============================================================");

    // `Ok(group_by_instrument(self.get_all()?))` — the ? short-circuits, so
    // grouping never runs on a failed fetch.
    let mut broken = InMemoryStore { rows: seed_points(), fail: true };
    match broken.index_by_instrument() {
        Ok(index) => println!("Ok({} instruments)", index.len()),
        Err(e) => println!("Err: {e}"),
    }
    println!();
    // The caller (SourceIndex::load) adds context on top:
    let with_context = broken
        .index_by_instrument()
        .map_err(|e| e.context("failed to load resampled points"));
    if let Err(e) = with_context {
        println!("with context: {e:#}");
    }
    println!();
    println!("  -> The domain stays error-agnostic (anyhow::Result); the");
    println!("     application layer names what was being loaded.");
}
