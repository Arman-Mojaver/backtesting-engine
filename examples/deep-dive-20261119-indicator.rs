//! Deep Dive: Indicator — the DB-configured indicator catalogue
//! Source: Code input — src/domain/indicator.rs
//!
//! An `Indicator` is one row of the Django-owned `indicator` table: a DB id
//! plus typed `IndicatorParameters`. The id is the identity used everywhere
//! downstream (signal-group cache key, processed-ids manifest, Parquet rows);
//! the parameters only decide which computation runs.

use std::env;

use anyhow::bail;
use backtesting_engine::domain::indicator::{
    Indicator, IndicatorParameters, IndicatorRepository, MovingAverageType, indicator_ids,
    unprocessed_indicators,
};
use rustc_hash::FxHashSet;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_parameters(),
        "2" => section_2_value_semantics(),
        "3" => section_3_unprocessed_happy_path(),
        "4" => section_4_unprocessed_edge_cases(),
        "5" => section_5_repository(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-indicator <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Parameters: the IndicatorParameters variants");
    println!("  2    - Value Semantics: Copy, Eq on parameters, identity by id");
    println!("  3    - Unprocessed Indicators: resuming a partially done cell");
    println!("  4    - Unprocessed Edge Cases: empty, all done, stray and duplicate ids");
    println!("  5    - Repository: the IndicatorRepository port and its caller");
}

// ---- Shared seed data ----
fn rsi(id: i32) -> Indicator {
    Indicator { id, parameters: IndicatorParameters::Rsi { n: 14, high_threshold: 70, low_threshold: 30 } }
}

fn catalogue() -> Vec<Indicator> {
    vec![
        rsi(101),
        Indicator {
            id: 102,
            parameters: IndicatorParameters::Macd {
                slow_type: MovingAverageType::Sma,
                slow_n: 26,
                fast_type: MovingAverageType::Ema,
                fast_n: 12,
            },
        },
        Indicator { id: 103, parameters: IndicatorParameters::AwesomeOscillator },
        Indicator { id: 104, parameters: IndicatorParameters::Roc { period: 5 } },
    ]
}

fn ids(values: &[i32]) -> FxHashSet<i32> {
    values.iter().copied().collect()
}

// ============================================================
// SECTION 1: Parameters - the IndicatorParameters variants
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_parameters() {
    println!("============================================================");
    println!("SECTION 1: Parameters - the IndicatorParameters variants");
    println!("============================================================");
    // Each variant carries exactly the knobs its computation needs; the PG
    // adapter rejects unknown JSON keys, so a variant is a closed schema.
    for indicator in catalogue() {
        println!("id={} -> {:?}", indicator.id, indicator.parameters);
    }
    println!();
    // Unit variants (AO, AC) have fixed periods by definition: no knobs.
    // MACD is the only one choosing an averaging kind per leg.
    let shape = |p: &IndicatorParameters| match p {
        IndicatorParameters::AwesomeOscillator | IndicatorParameters::AcceleratorDecelerator => "unit (no params)",
        IndicatorParameters::Macd { .. } => "MACD: MovingAverageType per leg",
        IndicatorParameters::Rsi { .. } => "threshold levels on 0-100 scale",
        _ => "period-style numeric params",
    };
    for indicator in catalogue() {
        println!("  {:>3} : {}", indicator.id, shape(&indicator.parameters));
    }
    println!();
    println!("DB type strings -> variants: macd rsi ao ac bears bulls cho chv dpo force osma roc trix vroc");
}

// ============================================================
// SECTION 2: Value Semantics - Copy, Eq on parameters, identity by id
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_value_semantics() {
    println!("============================================================");
    println!("SECTION 2: Value Semantics - Copy, Eq on parameters, identity by id");
    println!("============================================================");
    // Indicator is Copy (i32 + Copy enum): `unprocessed_indicators` can
    // `.copied()` out of a slice without any allocation per element.
    let a = rsi(101);
    let b = a; // a copy, `a` is still usable
    println!("a.id={} b.id={} (both alive after `let b = a`)", a.id, b.id);

    // Two rows with identical parameters are still two indicators.
    let twin = rsi(201);
    println!("same parameters? {}", a.parameters == twin.parameters);
    println!("same id?         {}", a.id == twin.id);
    println!();
    // Indicator deliberately has no PartialEq: "equal" would be ambiguous
    // (by id or by parameters?). Callers compare the field they mean.
    // let _ = a == twin; // <- does not compile
    println!("Indicator == Indicator does not compile; compare .id or .parameters");
    println!();
    println!("Why it matters: the signal-group cache keys on (indicator.id, instrument),");
    println!("so ids 101 and 201 are computed separately even with equal parameters.");
}

// ============================================================
// SECTION 3: Unprocessed Indicators - resuming a partially done cell
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_unprocessed_happy_path() {
    println!("============================================================");
    println!("SECTION 3: Unprocessed Indicators - resuming a partially done cell");
    println!("============================================================");
    // `strategy create` reads ids already persisted for the cell from the
    // Parquet manifest, then only runs the rest. Crash mid-run -> rerun resumes.
    let all = catalogue();
    let processed = ids(&[101, 103]);
    println!("catalogue ids : {:?}", indicator_ids(&all));
    println!("processed ids : {:?} (from manifest)", processed);

    let todo = unprocessed_indicators(&all, &processed);
    println!("unprocessed   : {:?}", indicator_ids(&todo));
    println!();
    println!("  -> filter keeps catalogue order (102 before 104)");
    println!("  -> FxHashSet lookup is O(1) per indicator");
    println!("  -> indicator_ids(&todo) becomes computed_indicator_ids,");
    println!("     appended to the manifest after strategies persist");
}

// ============================================================
// SECTION 4: Unprocessed Edge Cases - empty, all done, stray and duplicate ids
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_unprocessed_edge_cases() {
    println!("============================================================");
    println!("SECTION 4: Unprocessed Edge Cases - empty, all done, stray and duplicate ids");
    println!("============================================================");
    let all = catalogue();
    let case = |label: &str, input: &[Indicator], done: &[i32]| {
        let todo = unprocessed_indicators(input, &ids(done));
        println!("{label:<28} -> {:?}", indicator_ids(&todo));
    };
    case("empty catalogue", &[], &[101]);
    case("first run (nothing done)", &all, &[]);
    // Empty result is the "Nothing to do." early return in create.rs.
    case("all processed", &all, &[101, 102, 103, 104]);
    // Ids of indicators deleted from the DB linger in the manifest: harmless.
    case("stray processed id 999", &all, &[999, 102]);
    // The function does not dedupe: duplicates are filtered or kept together.
    let dup = [rsi(101), rsi(101), rsi(105)];
    case("duplicate id 101, not done", &dup, &[]);
    case("duplicate id 101, done", &dup, &[101]);
    println!();
    println!("  -> processed set is a filter only; it never adds indicators");
}

// ============================================================
// SECTION 5: Repository - the IndicatorRepository port and its caller
// (Copy the title above and search this file to find this code!)
// ============================================================
#[derive(Debug)]
struct InMemoryIndicators(Vec<Indicator>);

impl IndicatorRepository for InMemoryIndicators {
    fn get_all(&mut self) -> anyhow::Result<Vec<Indicator>> {
        Ok(self.0.clone())
    }
}

#[derive(Debug)]
struct BrokenIndicators;

impl IndicatorRepository for BrokenIndicators {
    fn get_all(&mut self) -> anyhow::Result<Vec<Indicator>> {
        bail!("unknown indicator type: stoch")
    }
}

fn section_5_repository() {
    println!("============================================================");
    println!("SECTION 5: Repository - the IndicatorRepository port and its caller");
    println!("============================================================");
    // Domain owns the trait; PgStore (infrastructure) implements it. `&mut self`
    // because a postgres::Client needs &mut to query.
    let mut ok = InMemoryIndicators(catalogue());
    println!("in-memory get_all -> {:?}", ok.get_all().map(|v| indicator_ids(&v)));
    // All-or-nothing: one malformed row fails the whole fetch (the PG adapter
    // bails in map_parameters), unlike compute errors, which skip one indicator.
    let mut broken = BrokenIndicators;
    println!("broken get_all    -> {:?}", broken.get_all().map(|v| v.len()));
    println!();
    println!("Real caller (needs a DB; run it yourself, this script does not):");
    println!("  cargo run --bin bte -- strategy create \\");
    println!("    --money_management_strategy_id 390 --instrument EURUSD");
    println!("Flow: fetch_source_data -> IndicatorRepository::get_all");
    println!("   -> unprocessed_indicators -> run_strategies -> persist + ids");
}
