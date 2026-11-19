//! Deep Dive: OperationPoint — precomputed trade outcomes per bar
//! Source: Code input — src/domain/operation_point.rs
//!
//! An OperationPoint answers "if a trade opened at this timestamp, how did it
//! end?" for one (instrument, money-management strategy) Cell. Long and short
//! outcomes live in separate tables. The engine never simulates trades itself:
//! it looks up signal timestamps in these points. This file provides the
//! lookup maps, the backtest date range, and the per-Cell grouping used at load.

use std::env;

use backtesting_engine::domain::cell::Cell;
use backtesting_engine::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, OperationPoints, points_by_timestamp,
};
use itertools::Itertools;

const DAY: i64 = 86_400;
const JAN_02_2024: i64 = 1_704_153_600;

fn op(id: i32, instrument: &str, mms_id: i32, timestamp: i64, result: i32) -> OperationPoint {
    OperationPoint {
        id,
        instrument: instrument.into(),
        money_management_strategy_id: mms_id,
        timestamp,
        result,
        tp: 30,
        sl: 20,
        risk: 0.02,
    }
}

// Four consecutive daily bars for EURUSD / mms 1, long and short in lockstep.
const BAR_TIMESTAMPS: [i64; 4] = [
    JAN_02_2024,
    JAN_02_2024 + DAY,
    JAN_02_2024 + 2 * DAY,
    JAN_02_2024 + 3 * DAY,
];

fn long_points() -> Vec<OperationPoint> {
    let ids = [100, 101, 102, 103];
    let results = [1, -1, 1, 1];
    (ids.into_iter().zip(BAR_TIMESTAMPS).zip(results))
        .map(|((id, ts), result)| op(id, "EURUSD", 1, ts, result))
        .collect()
}

fn short_points() -> Vec<OperationPoint> {
    let ids = [200, 201, 202, 203];
    let results = [-1, 1, -1, -1];
    (ids.into_iter().zip(BAR_TIMESTAMPS).zip(results))
        .map(|((id, ts), result)| op(id, "EURUSD", 1, ts, result))
        .collect()
}

/// Stands in for PgStore so the trait's default `index_by_cell` (and the
/// private `group_by_cell` behind it) can be driven without a database.
struct InMemoryLongStore(Vec<OperationPoint>);

impl LongOperationPointRepository for InMemoryLongStore {
    fn get_all(&mut self) -> anyhow::Result<Vec<OperationPoint>> {
        Ok(self.0.clone())
    }

    fn get_by_mms_id_and_instrument(
        &mut self,
        mms_id: i32,
        instrument: &str,
    ) -> anyhow::Result<Vec<OperationPoint>> {
        Ok(self
            .0
            .iter()
            .filter(|p| p.money_management_strategy_id == mms_id && p.instrument == instrument)
            .cloned()
            .collect())
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map_or("list", String::as_str) {
        "1" => section_1_anatomy(),
        "2" => section_2_points_by_timestamp(),
        "3" => section_3_start_end_timestamps(),
        "4" => section_4_index_by_cell(),
        "5" => section_5_signal_lookup(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-operation-point <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Anatomy: OperationPoint fields and the OperationPoints view");
    println!("  2    - Lookup Maps: points_by_timestamp, empty input, duplicate last-wins");
    println!("  3    - Backtest Range: start_end_timestamps from the long side only");
    println!("  4    - Load Grouping: index_by_cell runs, interleaving, empty input");
    println!("  5    - Signal Lookup: how strategy_generator consumes the maps");
}

// ============================================================
// SECTION 1: Anatomy - OperationPoint Fields and the OperationPoints View
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_anatomy() {
    println!("============================================================");
    println!("SECTION 1: Anatomy - OperationPoint Fields and the OperationPoints View");
    println!("============================================================");

    let long = long_points();
    let short = short_points();
    let Some(first) = long.first() else { return };
    println!(
        "id={} instrument={} mms_id={}",
        first.id, first.instrument, first.money_management_strategy_id
    );
    println!("timestamp={} (bar timestamp, unix secs)", first.timestamp);
    println!(
        "result={}  (+1 = winning trade, -1 = losing trade)",
        first.result
    );
    println!("tp={} sl={} risk={}", first.tp, first.sl, first.risk);
    println!();

    // OperationPoints owns nothing: it borrows two slices, so it is Copy and
    // cheap to pass into the parallel strategy loop.
    let ops = OperationPoints {
        long_operation_points: &long,
        short_operation_points: &short,
    };
    let copied = ops; // Copy, not move — `ops` stays usable
    println!(
        "long={} short={} (copy still sees long={})",
        ops.long_operation_points.len(),
        ops.short_operation_points.len(),
        copied.long_operation_points.len()
    );
    println!();

    // Same timestamp, opposite direction, independent outcome.
    for (l, s) in long.iter().zip(&short) {
        println!(
            "ts={}  long result={:>2}  short result={:>2}",
            l.timestamp, l.result, s.result
        );
    }
}

// ============================================================
// SECTION 2: Lookup Maps - points_by_timestamp, Empty Input, Duplicate Last-Wins
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_points_by_timestamp() {
    println!("============================================================");
    println!("SECTION 2: Lookup Maps - points_by_timestamp, Empty Input, Duplicate Last-Wins");
    println!("============================================================");

    let long = long_points();
    let short = short_points();
    let ops = OperationPoints {
        long_operation_points: &long,
        short_operation_points: &short,
    };

    // Values are &OperationPoint borrowed from the slice: no cloning.
    let long_map = ops.long_points_by_timestamp();
    let short_map = ops.short_points_by_timestamp();
    let ts = JAN_02_2024 + DAY;
    println!(
        "long_map.len()={} short_map.len()={}",
        long_map.len(),
        short_map.len()
    );
    println!("long_map[{ts}].id  = {:?}", long_map.get(&ts).map(|p| p.id));
    println!(
        "short_map[{ts}].id = {:?}",
        short_map.get(&ts).map(|p| p.id)
    );
    println!("get(unknown ts)     = {:?}", long_map.get(&0).map(|p| p.id));
    println!();

    // Edge: empty slice -> empty map (no special-casing needed).
    println!("empty input -> len {}", points_by_timestamp(&[]).len());

    // Edge: duplicate timestamps silently collapse; the LAST point wins
    // because collect() inserts in order and overwrites.
    let dupes = [op(1, "EURUSD", 1, 1_000, 1), op(2, "EURUSD", 1, 1_000, -1)];
    let map = points_by_timestamp(&dupes);
    println!(
        "dupes: 2 points, 1 ts -> len {} id {:?}",
        map.len(),
        map.get(&1_000).map(|p| p.id)
    );
    println!("  -> Upstream guarantees one point per bar, so this is never hit");
    println!("     in practice; nothing here detects it either.");
}

// ============================================================
// SECTION 3: Backtest Range - start_end_timestamps From the Long Side Only
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_start_end_timestamps() {
    println!("============================================================");
    println!("SECTION 3: Backtest Range - start_end_timestamps From the Long Side Only");
    println!("============================================================");

    // The DB query orders by (instrument, mms_id) only, NOT timestamp, so
    // first()/last() would be wrong: minmax() scans for the true bounds.
    let mut shuffled = long_points();
    shuffled.swap(0, 3);
    println!(
        "input ts order: {}",
        shuffled.iter().map(|p| p.timestamp).join(", ")
    );
    let ops = OperationPoints {
        long_operation_points: &shuffled,
        short_operation_points: &[],
    };
    println!("unsorted   -> {:?}", ops.start_end_timestamps());

    // Edge: a single point is both start and end (MinMax::OneElement).
    let one = [op(1, "EURUSD", 1, 42, 1)];
    let ops = OperationPoints {
        long_operation_points: &one,
        short_operation_points: &[],
    };
    println!("one point  -> {:?}", ops.start_end_timestamps());

    // Edge: short points are ignored entirely, even if they extend further.
    let long = [op(1, "EURUSD", 1, 200, 1), op(2, "EURUSD", 1, 300, 1)];
    let short = [op(3, "EURUSD", 1, 100, -1), op(4, "EURUSD", 1, 400, -1)];
    let ops = OperationPoints {
        long_operation_points: &long,
        short_operation_points: &short,
    };
    println!(
        "short wider-> {:?}  (short 100..400 ignored)",
        ops.start_end_timestamps()
    );

    // Error path: no long points -> None. strategy_runner turns that into
    // anyhow!("long operation points should never be empty").
    let ops = OperationPoints {
        long_operation_points: &[],
        short_operation_points: &short,
    };
    println!("empty long -> {:?}", ops.start_end_timestamps());
    println!();
    println!("  -> Long/short are generated in lockstep, so one side bounds both;");
    println!("     the range feeds annual_operation_count's per-year scaling.");
}

// ============================================================
// SECTION 4: Load Grouping - index_by_cell Runs, Interleaving, Empty Input
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_index_by_cell() {
    println!("============================================================");
    println!("SECTION 4: Load Grouping - index_by_cell Runs, Interleaving, Empty Input");
    println!("============================================================");

    // Shape PgStore::get_all returns: sorted by (instrument, mms_id), so each
    // Cell is one contiguous run. group_by_cell builds one Cell key (one
    // instrument String clone) per run instead of per point.
    let sorted = vec![
        op(1, "EURUSD", 1, 1_000, 1),
        op(2, "EURUSD", 1, 1_001, -1),
        op(3, "EURUSD", 2, 1_000, 1),
        op(4, "USDCAD", 1, 1_000, 1),
    ];
    print_index("sorted (3 runs)", InMemoryLongStore(sorted));

    // Interleaved input: EURUSD appears in two separate runs. The Occupied
    // branch of insert_run extends the existing Vec, keeping input order.
    let interleaved = vec![
        op(1, "EURUSD", 1, 1_000, 1),
        op(2, "USDCAD", 1, 1_000, 1),
        op(3, "EURUSD", 1, 1_001, 1),
    ];
    print_index("interleaved (3 runs)", InMemoryLongStore(interleaved));

    // Edge: no points -> the final insert_run sees an empty run and returns
    // early via let-else, so no empty-Vec entry is created.
    print_index("empty", InMemoryLongStore(vec![]));

    // The non-default trait method filters to a single Cell instead.
    let mut store = InMemoryLongStore(long_points());
    let one_cell = store
        .get_by_mms_id_and_instrument(1, "EURUSD")
        .map(|v| v.len());
    println!("get_by_mms_id_and_instrument(1, EURUSD) -> {one_cell:?} points");
}

fn print_index(label: &str, mut store: InMemoryLongStore) {
    let index = match store.index_by_cell() {
        Ok(index) => index,
        Err(e) => return println!("{label}: error {e}"),
    };
    println!("{label}: {} cells", index.len());
    for (cell, points) in index.iter().sorted_by(|a, b| a.0.cmp(b.0)) {
        println!(
            "  {cell} -> ids {:?}",
            points.iter().map(|p| p.id).collect_vec()
        );
    }
}

// ============================================================
// SECTION 5: Signal Lookup - How strategy_generator Consumes the Maps
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_signal_lookup() {
    println!("============================================================");
    println!("SECTION 5: Signal Lookup - How strategy_generator Consumes the Maps");
    println!("============================================================");

    let long = long_points();
    let short = short_points();
    let ops = OperationPoints {
        long_operation_points: &long,
        short_operation_points: &short,
    };
    let cell = Cell::new("EURUSD", 1);

    // Maps are built ONCE per Cell, then shared by every SignalGroup in the
    // rayon loop: O(1) lookups instead of scanning slices per signal.
    let long_map = ops.long_points_by_timestamp();
    let short_map = ops.short_points_by_timestamp();

    // Seed signals: one falls outside the operation-point range (ATR warm-up
    // bars have no ops), so it simply never matches.
    let long_signals = [JAN_02_2024, JAN_02_2024 + 2 * DAY, JAN_02_2024 - DAY];
    let short_signals = [JAN_02_2024 + DAY];

    println!("cell {cell}");
    for ts in long_signals {
        let hit = long_map.get(&ts).map(|p| p.result);
        println!("  long  signal ts={ts} -> result {hit:?}");
    }
    for ts in short_signals {
        let hit = short_map.get(&ts).map(|p| p.result);
        println!("  short signal ts={ts} -> result {hit:?}");
    }

    // Mirrors the private matched_trade_count in strategy_generator.rs.
    let long_matched = long_signals
        .iter()
        .filter(|ts| long_map.contains_key(ts))
        .count();
    let short_matched = short_signals
        .iter()
        .filter(|ts| short_map.contains_key(ts))
        .count();
    println!();
    println!("matched long  = {long_matched} of {}", long_signals.len());
    println!("matched short = {short_matched} of {}", short_signals.len());
    println!("  -> This count drives the annual-activity filter before any metrics.");
}
