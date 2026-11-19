//! Deep Dive: thresholds — level-cross signal detection for RSI-like indicators
//! Source: Code input — src/domain/signal/thresholds.rs
//!
//! detect_signals turns a ThresholdBuffer (indicator values + high/low levels)
//! into (long_timestamps, short_timestamps). The core, level_crosses, is also
//! reused by oscillator::detect_signals with both levels at 0.0.

use std::env;

use backtesting_engine::domain::signal::thresholds::{detect_signals, level_crosses};
use backtesting_engine::domain::signal::{IndicatorValue, ThresholdBuffer};

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_level_mapping(),
        "2" => section_2_touching_the_level(),
        "3" => section_3_independent_sides(),
        "4" => section_4_edge_inputs(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-thresholds <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Level Mapping: low level drives longs, high level drives shorts");
    println!("  2    - Touching the Level: equal values are skipped, not crossed");
    println!("  3    - Independent Sides: no pairing, no alternation, oscillator reuse");
    println!("  4    - Edge Inputs: empty, single value, inverted levels, NaN");
}

// Index-as-timestamp keeps the output readable; the real buffer carries
// resampled-point timestamps.
fn series(values: &[f64]) -> Vec<IndicatorValue> {
    (0..)
        .zip(values)
        .map(|(timestamp, &value)| IndicatorValue { timestamp, value })
        .collect()
}

fn rsi_buffer(values: &[f64]) -> ThresholdBuffer {
    ThresholdBuffer { items: series(values), high_threshold: 70.0, low_threshold: 30.0 }
}

fn show(label: &str, values: &[f64], result: &(Vec<i64>, Vec<i64>)) {
    println!("{label:<22} {values:?}");
    println!("{:<22} longs at {:?}, shorts at {:?}", "", result.0, result.1);
}

// ============================================================
// SECTION 1: Level Mapping - Low Level Drives Longs, High Level Drives Shorts
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_level_mapping() {
    println!("============================================================");
    println!("SECTION 1: Level Mapping - Low Level Drives Longs, High Level Drives Shorts");
    println!("============================================================");

    let rsi = [45.0, 25.0, 35.0, 75.0, 65.0, 50.0];
    println!("levels: low = 30, high = 70   (index = timestamp)");
    show("detect_signals", &rsi, &detect_signals(&rsi_buffer(&rsi)));
    println!();

    // Same call by hand: detect_signals passes low as long_level, high as short_level.
    let by_hand = level_crosses(&series(&rsi), 30.0, 70.0);
    show("level_crosses(30, 70)", &rsi, &by_hand);
    let swapped = level_crosses(&series(&rsi), 70.0, 30.0);
    show("level_crosses(70, 30)", &rsi, &swapped);
    println!();
    println!("  -> long  = crosses ABOVE the low level  (oversold recovery): 25 → 35 at t=2");
    println!("  -> short = crosses BELOW the high level (overbought fade):   75 → 65 at t=4");
    println!("  -> the timestamp is the value AFTER the cross (curr), never prev;");
    println!("     create_signal_group then shifts it one more resampled point forward");
    println!("  -> swapping the arguments turns it into a breakout strategy instead:");
    println!("     long when breaking above 70 (t=3), short below 30 (t=1)");
}

// ============================================================
// SECTION 2: Touching the Level - Equal Values Are Skipped, Not Crossed
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_touching_the_level() {
    println!("============================================================");
    println!("SECTION 2: Touching the Level - Equal Values Are Skipped, Not Crossed");
    println!("============================================================");

    let cases: [(&str, &[f64]); 4] = [
        ("touch 30, bounce up", &[40.0, 30.0, 35.0]),
        ("from below via 30", &[20.0, 30.0, 40.0]),
        ("from below, back down", &[20.0, 30.0, 20.0]),
        ("parked on 30 twice", &[20.0, 30.0, 30.0, 40.0]),
    ];
    for (label, values) in cases {
        show(label, values, &detect_signals(&rsi_buffer(values)));
    }
    println!();
    println!("  -> .filter(value != level) runs BEFORE tuple_windows, so the window");
    println!("     pairs the last value off the level with the next one off it");
    println!("  -> 20, 30, 40 becomes the pair (20, 40): a real cross, stamped at 40");
    println!("  -> 40, 30, 35 becomes (40, 35): never went below, no long");
    println!("  -> without the filter, strict < / > would miss 20→30→40 entirely");
    println!("  -> each side filters its own level: 30 is kept for the 70 check");
}

// ============================================================
// SECTION 3: Independent Sides - No Pairing, No Alternation, Oscillator Reuse
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_independent_sides() {
    println!("============================================================");
    println!("SECTION 3: Independent Sides - No Pairing, No Alternation, Oscillator Reuse");
    println!("============================================================");

    let cases: [(&str, &[f64]); 3] = [
        ("jump 20 → 80", &[20.0, 80.0]),
        ("two longs, no short", &[10.0, 40.0, 15.0, 45.0]),
        ("crash 80 → 20", &[80.0, 20.0]),
    ];
    for (label, values) in cases {
        show(label, values, &detect_signals(&rsi_buffer(values)));
    }
    println!();

    // oscillator::detect_signals is exactly level_crosses(items, 0.0, 0.0).
    let ao = [-1.0, 1.0, 0.0, -2.0];
    show("oscillator (0.0, 0.0)", &ao, &level_crosses(&series(&ao), 0.0, 0.0));
    println!();
    println!("  -> crosses_above and crosses_below scan the series separately;");
    println!("     a long does not wait for a short, nor vice versa");
    println!("  -> 20 → 80 crosses above 30 (long) but never crosses below 70");
    println!("  -> with equal levels, every sign change is exactly one signal");
}

// ============================================================
// SECTION 4: Edge Inputs - Empty, Single Value, Inverted Levels, NaN
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_edge_inputs() {
    println!("============================================================");
    println!("SECTION 4: Edge Inputs - Empty, Single Value, Inverted Levels, NaN");
    println!("============================================================");

    show("empty", &[], &detect_signals(&rsi_buffer(&[])));
    show("single value", &[25.0], &detect_signals(&rsi_buffer(&[25.0])));

    let wave = [20.0, 50.0, 80.0, 60.0, 20.0];
    let inverted = ThresholdBuffer { items: series(&wave), high_threshold: 30.0, low_threshold: 70.0 };
    show("inverted (low 70)", &wave, &detect_signals(&inverted));

    let gap = [20.0, f64::NAN, 40.0];
    show("NaN between", &gap, &detect_signals(&rsi_buffer(&gap)));
    println!();
    println!("  -> tuple_windows needs two items; fewer yields no windows, no panic");
    println!("  -> nothing here checks low < high: inverted levels silently act");
    println!("     as a breakout rule (compare section 1's swapped call)");
    println!("  -> NaN != level is true, so NaN survives the filter, and every");
    println!("     comparison with NaN is false: the 20 → 40 cross is lost");
}
