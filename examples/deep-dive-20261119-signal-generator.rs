//! Deep Dive: create_signal_groups (signal generator)
//! Source: Code input — src/application/signal_generator.rs
//!
//! For one instrument's daily resampled points, turns every Indicator into a
//! SignalGroup (long/short entry timestamps) in parallel. Per indicator:
//!   create_indicator_output (buffers) -> detect crosses -> shift to next point
//! The per-indicator step is memoized process-wide on (indicator_id, instrument)
//! and a misconfigured indicator is skipped with one warn, never aborting.
//!
//! `create_signal_group` is private, so every section goes through the public
//! `create_signal_groups`. The memo is global: each section uses its own ids and
//! prints cache-stat DELTAS. Each section runs in its own process.

use std::env;

use backtesting_engine::application::signal_generator::{
    SignalGroupCacheStats, create_signal_groups, instrument_of, signal_group_cache_stats,
};
use backtesting_engine::domain::indicator::{Indicator, IndicatorParameters, MovingAverageType};
use backtesting_engine::domain::indicators::create_indicator_output;
use backtesting_engine::domain::resampled_point::ResampledPointD1;
use simplelog::{Config, LevelFilter, SimpleLogger};

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map_or("list", String::as_str) {
        "1" => section_1_happy_path(),
        "2" => section_2_macd_crossover(),
        "3" => section_3_skip_not_abort(),
        "4" => section_4_cache_key(),
        "5" => section_5_order_and_stats(),
        "6" => section_6_edge_cases(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-signal-generator <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Happy Path: RSI points become one SignalGroup");
    println!("  2    - MACD Crossover: dip goes long, spike goes short");
    println!("  3    - Skip, Not Abort: a bad indicator warns once and vanishes");
    println!("  4    - Cache Key: (indicator_id, instrument), never the points");
    println!("  5    - Order and Stats: rayon keeps input order, stats count calls");
    println!("  6    - Edge Cases: no indicators, no points, instrument_of");
}

// ---- Shared seed data ----

// Same shape as the unit tests' helper: flat OHLC = close, ts = (i + 1) * 1000.
fn points(instrument: &str, closes: &[f64]) -> Vec<ResampledPointD1> {
    closes
        .iter()
        .zip(1..)
        .map(|(&close, n)| ResampledPointD1 {
            id: n,
            instrument: instrument.into(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 1000,
            timestamp: i64::from(n) * 1000,
        })
        .collect()
}

const DIP: [f64; 14] = [100., 100., 100., 100., 100., 100., 100., 100., 100., 82., 100., 100., 100., 100.];
const SPIKE: [f64; 14] = [100., 100., 100., 100., 100., 100., 100., 100., 100., 118., 100., 100., 100., 100.];

fn rsi(id: i32, n: i32) -> Indicator {
    Indicator { id, parameters: IndicatorParameters::Rsi { n, high_threshold: 70, low_threshold: 30 } }
}

// fast EMA(1) is the close itself; slow SMA(2): line = half the close change.
fn macd(id: i32, fast_n: i32, slow_n: i32) -> Indicator {
    Indicator {
        id,
        parameters: IndicatorParameters::Macd {
            slow_type: MovingAverageType::Sma,
            slow_n,
            fast_type: MovingAverageType::Ema,
            fast_n,
        },
    }
}

fn delta(before: SignalGroupCacheStats) -> String {
    let now = signal_group_cache_stats();
    format!("+{} hits, +{} misses, {} entries", now.hits - before.hits, now.misses - before.misses, now.entries)
}

// ============================================================
// SECTION 1: Happy Path - RSI Points Become One SignalGroup
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_happy_path() {
    println!("============================================================");
    println!("SECTION 1: Happy Path - RSI Points Become One SignalGroup");
    println!("============================================================");

    let pts = points("EURUSD", &[5.0, 4.0, 3.0, 4.0, 4.0]);
    let indicator = rsi(7001, 2);
    println!("closes 5,4,3,4,4 at ts 1000..5000, RSI n=2, thresholds 70/30");

    // Step 1 (inside create_signal_group): parameters -> typed buffer.
    let output = create_indicator_output(&indicator.parameters, &pts).unwrap();
    println!("step 1: create_indicator_output -> {}", format!("{output:?}").split_whitespace().next().unwrap());
    println!("        RSI ts 3000 = 0 (two losses), ts 4000 = 50 (one each)");

    // Step 2: 0 -> 50 crosses UP through the low threshold 30 at ts 4000.
    // Step 3: the entry is shifted to the NEXT resampled point, ts 5000:
    //         the signal is only known once the ts-4000 bar has closed.
    let groups = create_signal_groups(&[indicator], &pts);
    let g = &groups[0];
    println!("step 2+3: {g:?}");
    println!();
    println!("  -> indicator_id stamps the group so the Strategy can trace back.");
    println!("  -> Timestamps, not indices: strategy_generator looks them up in");
    println!("     the operation point maps.");
}

// ============================================================
// SECTION 2: MACD Crossover - Dip Goes Long, Spike Goes Short
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_macd_crossover() {
    println!("============================================================");
    println!("SECTION 2: MACD Crossover - Dip Goes Long, Spike Goes Short");
    println!("============================================================");

    // Different ids per instrument series: the memo key is (id, instrument).
    let dip = create_signal_groups(&[macd(9001, 1, 2)], &points("EURUSD", &DIP));
    let spike = create_signal_groups(&[macd(9002, 1, 2)], &points("EURUSD", &SPIKE));
    println!("dip   (100 -> 82 at ts 10000):  long={:?} short={:?}", dip[0].long_signals, dip[0].short_signals);
    println!("spike (100 -> 118 at ts 10000): long={:?} short={:?}", spike[0].long_signals, spike[0].short_signals);
    println!();
    println!("dip trace:  line   = [.., -9 @9000, +9 @10000, 0, 0, 0]");
    println!("            signal = SMA(line, 9) = [-1, 0, 0, 0, 0] from ts 10000");
    println!("            diff   = [-8, +9, 0, ..] -> strict - to + at ts 11000");
    println!("            entry on the next point -> ts 12000");
    println!();
    println!("  -> Threshold (RSI) and Crossover (MACD) outputs are different");
    println!("     IndicatorOutput variants; create_signal_groups never knows which.");
}

// ============================================================
// SECTION 3: Skip, Not Abort - A Bad Indicator Warns Once and Vanishes
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_skip_not_abort() {
    println!("============================================================");
    println!("SECTION 3: Skip, Not Abort - A Bad Indicator Warns Once and Vanishes");
    println!("============================================================");
    SimpleLogger::init(LevelFilter::Warn, Config::default()).unwrap();

    let pts = points("EURUSD", &[5.0, 4.0, 3.0, 4.0, 4.0]);
    // fast 26 >= slow 12: rejected by create_indicator_output.
    let indicators = vec![macd(100, 26, 12), rsi(101, 2)];
    let before = signal_group_cache_stats();

    println!("call 1 (2 indicators):");
    let first = create_signal_groups(&indicators, &pts);
    println!("  -> {} group(s), ids {:?}", first.len(), first.iter().map(|g| g.indicator_id).collect::<Vec<_>>());

    println!("call 2 (same cell again, e.g. the next mms):");
    let second = create_signal_groups(&indicators, &pts);
    println!("  -> {} group(s), no new warn above", second.len());
    println!("cache: {}", delta(before));
    println!();
    println!("  -> cache_none = true memoizes the None too, so a bad row costs");
    println!("     one warn per (indicator, instrument), not one per cell.");
    println!("  -> filter_map drops it; the good RSI is unaffected.");
}

// ============================================================
// SECTION 4: Cache Key - (indicator_id, instrument), Never the Points
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_cache_key() {
    println!("============================================================");
    println!("SECTION 4: Cache Key - (indicator_id, instrument), Never the Points");
    println!("============================================================");

    let indicators = [macd(9101, 1, 2)];
    let before = signal_group_cache_stats();

    // Same id, different instrument -> different keys -> both computed.
    let eur = create_signal_groups(&indicators, &points("EURUSD", &DIP));
    let gbp = create_signal_groups(&indicators, &points("GBPUSD", &SPIKE));
    println!("EURUSD dip:   {} long={:?} short={:?}", eur[0].instrument, eur[0].long_signals, eur[0].short_signals);
    println!("GBPUSD spike: {} long={:?} short={:?}", gbp[0].instrument, gbp[0].long_signals, gbp[0].short_signals);
    println!("cache: {}", delta(before));

    // The trade-off made visible: same key, DIFFERENT points -> stale hit.
    let before = signal_group_cache_stats();
    let stale = create_signal_groups(&indicators, &points("EURUSD", &SPIKE));
    println!();
    println!("EURUSD spike (same key!): long={:?} short={:?}", stale[0].long_signals, stale[0].short_signals);
    println!("cache: {}", delta(before));
    println!();
    println!("  -> A hit returns the dip's long, not the spike's short.");
    println!("  -> Safe in the engine: parameters are immutable per id and one");
    println!("     instrument's points never change within a process. Keying on");
    println!("     the identity avoids hashing thousands of points per lookup.");
}

// ============================================================
// SECTION 5: Order and Stats - Rayon Keeps Input Order, Stats Count Calls
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_order_and_stats() {
    println!("============================================================");
    println!("SECTION 5: Order and Stats - Rayon Keeps Input Order, Stats Count Calls");
    println!("============================================================");

    let ids = [9207, 9201, 9214, 9203, 9210, 9216, 9202, 9209, 9205, 9213, 9204, 9215, 9208, 9211, 9206, 9212];
    let indicators: Vec<Indicator> = ids.iter().map(|&id| rsi(id, 3)).collect();
    let pts = points("EURUSD", &[1.0, 2.0, 1.0, 2.0, 1.0]);

    let before = signal_group_cache_stats();
    let out = create_signal_groups(&indicators, &pts);
    println!("input ids:  {ids:?}");
    println!("output ids: {:?}", out.iter().map(|g| g.indicator_id).collect::<Vec<_>>());
    println!("cold run:   {}", delta(before));

    let before = signal_group_cache_stats();
    create_signal_groups(&indicators, &pts);
    println!("warm run:   {}", delta(before));
    println!();
    println!("  -> par_iter over a slice is an indexed source: filter_map +");
    println!("     collect rebuild the input order whatever thread ran first.");
    println!("  -> Identical parameters under 16 ids are 16 entries: the key is");
    println!("     the id, not the parameters.");
    println!("  -> strategy_batch logs these totals once at the end of a batch.");
}

// ============================================================
// SECTION 6: Edge Cases - No Indicators, No Points, instrument_of
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_edge_cases() {
    println!("============================================================");
    println!("SECTION 6: Edge Cases - No Indicators, No Points, instrument_of");
    println!("============================================================");
    SimpleLogger::init(LevelFilter::Warn, Config::default()).unwrap();

    let pts = points("EURUSD", &[5.0, 4.0, 3.0]);
    println!("instrument_of(3 EURUSD points) = {:?}", instrument_of(&pts));
    println!("instrument_of(empty)           = {:?}", instrument_of(&[]));

    let before = signal_group_cache_stats();
    println!("no indicators -> {} groups ({})", create_signal_groups(&[], &pts).len(), delta(before));

    println!("no points (warn expected):");
    let before = signal_group_cache_stats();
    let out = create_signal_groups(&[rsi(9401, 2)], &[]);
    println!("  -> {} groups ({})", out.len(), delta(before));

    // Only ZERO points is an error; a short series just yields no RSI windows.
    println!("3 points, RSI n=14 (no warn):");
    let out = create_signal_groups(&[rsi(9402, 14)], &pts);
    println!("  -> {} group, long={:?} short={:?}", out.len(), out[0].long_signals, out[0].short_signals);
    println!("  -> kept as an empty group; strategy_generator's activity gate drops it.");
    println!();
    println!("  -> instrument_of returns \"\" instead of an Option: it only feeds");
    println!("     the cache key and log lines, and empty input already fails in");
    println!("     create_indicator_output with NotEnoughData.");
}
