//! Deep Dive: oscillator — zero-line sign-flip signal detection
//! Source: Code input — src/domain/signal/oscillator.rs
//!
//! Oscillator indicators (ROC, VROC, CHO, CHV, ...) produce an OscillatorBuffer
//! centred on zero. detect_signals is a one-liner: thresholds::level_crosses
//! with both levels at 0.0, so a long is a cross up through zero and a short is
//! a cross down through zero.

use std::env;

use backtesting_engine::domain::resampled_point::ResampledPointD1;
use backtesting_engine::domain::signal::oscillator::detect_signals;
use backtesting_engine::domain::signal::thresholds::level_crosses;
use backtesting_engine::domain::signal::{IndicatorOutput, IndicatorValue, OscillatorBuffer};

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_sign_flips(),
        "2" => section_2_zero_is_not_a_side(),
        "3" => section_3_every_flip_signals(),
        "4" => section_4_edge_inputs(),
        "5" => section_5_into_a_signal_group(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-oscillator <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Sign Flips: negative-to-positive is long, positive-to-negative is short");
    println!("  2    - Zero Is Not a Side: zero values are skipped, not crossed");
    println!("  3    - Every Flip Signals: no pairing, no alternation, no magnitude filter");
    println!("  4    - Edge Inputs: empty, single value, all zeros, -0.0, NaN");
    println!("  5    - Into a SignalGroup: shift to the next resampled point");
}

// Index-as-timestamp keeps the output readable; real buffers carry
// resampled-point timestamps.
fn buffer(values: &[f64]) -> OscillatorBuffer {
    OscillatorBuffer {
        items: (0..)
            .zip(values)
            .map(|(timestamp, &value)| IndicatorValue { timestamp, value })
            .collect(),
    }
}

fn show(label: &str, values: &[f64]) {
    let (longs, shorts) = detect_signals(&buffer(values));
    println!("{label:<24} {values:?}");
    println!("{:<24} longs at {longs:?}, shorts at {shorts:?}", "");
}

// ============================================================
// SECTION 1: Sign Flips - Negative-to-Positive Is Long, Positive-to-Negative Is Short
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_sign_flips() {
    println!("============================================================");
    println!("SECTION 1: Sign Flips - Negative-to-Positive Is Long, Positive-to-Negative Is Short");
    println!("============================================================");
    show("up through zero", &[-2.0, 2.0]);
    show("down through zero", &[0.7, -0.9]);
    show("stays positive", &[1.5, 2.3]);
    show("stays negative", &[-0.6, -1.2]);
    println!();
    println!("  -> The signal is stamped on the bar AFTER the flip (curr in the window).");

    // Equivalence: the whole function is level_crosses with both levels at 0.0.
    let items = buffer(&[-0.4, 0.3, -0.8, 0.9]).items;
    let via_oscillator = detect_signals(&OscillatorBuffer { items: items.clone() });
    let via_level_crosses = level_crosses(&items, 0.0, 0.0);
    println!("detect_signals == level_crosses(items, 0.0, 0.0)? {}", via_oscillator == via_level_crosses);
}

// ============================================================
// SECTION 2: Zero Is Not a Side - Zero Values Are Skipped, Not Crossed
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_zero_is_not_a_side() {
    println!("============================================================");
    println!("SECTION 2: Zero Is Not a Side - Zero Values Are Skipped, Not Crossed");
    println!("============================================================");
    // crosses_above/below filter out values == level BEFORE pairing neighbours,
    // so a run of zeros collapses and the bars on either side become adjacent.
    show("through one zero", &[-1.0, 0.0, 1.0]);
    show("through several zeros", &[-1.0, 0.0, 0.0, 1.0]);
    show("down through zero", &[1.0, 0.0, -1.0]);
    show("touch zero, fall back", &[-1.0, 0.0, -1.0]);
    show("touch zero, rise back", &[1.0, 0.0, 1.0]);
    println!();
    println!("  -> Crossing via zero signals on the first non-zero bar past it (idx 2 / 3),");
    println!("     never on the zero bar itself.");
    println!("  -> Touching zero and returning to the same side is not a cross.");
}

// ============================================================
// SECTION 3: Every Flip Signals - No Pairing, No Alternation, No Magnitude Filter
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_every_flip_signals() {
    println!("============================================================");
    println!("SECTION 3: Every Flip Signals - No Pairing, No Alternation, No Magnitude Filter");
    println!("============================================================");
    show("flip on every bar", &[0.3, -0.4, 0.5, -0.6, 0.7]);
    show("starts above zero", &[0.6, 1.2, -1.4, 0.5]);
    show("tiny flips count too", &[-0.0001, 0.0001, -0.0001]);
    show("big moves, same side", &[0.1, 90.0, 0.2, 75.0]);
    println!();
    println!("  -> Longs and shorts are computed independently; the first signal can");
    println!("     be either side, depending on where the series starts.");
    println!("  -> Only the sign matters: a 0.0002 wiggle and a 90-point swing weigh");
    println!("     the same, so noisy oscillators near zero emit many signals.");
}

// ============================================================
// SECTION 4: Edge Inputs - Empty, Single Value, All Zeros, -0.0, NaN
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_edge_inputs() {
    println!("============================================================");
    println!("SECTION 4: Edge Inputs - Empty, Single Value, All Zeros, -0.0, NaN");
    println!("============================================================");
    show("empty", &[]);
    show("single value", &[1.2]);
    show("all zeros", &[0.0, 0.0, 0.0]);
    show("-0.0 between sides", &[-1.0, -0.0, 1.0]);
    show("NaN between sides", &[-1.0, f64::NAN, 1.0]);
    show("NaN then recovery", &[-1.0, f64::NAN, -1.0, 1.0]);
    println!();
    println!("  -> Fewer than two non-zero values: no window, no signal.");
    println!("  -> -0.0 == 0.0 in IEEE 754, so it is skipped like a zero.");
    println!("  -> NaN != 0.0, so it is NOT skipped; every comparison with NaN is");
    println!("     false, so it swallows the cross it sits in. The next clean flip");
    println!("     still signals.");
}

fn resampled_point(id: i32, timestamp: i64) -> ResampledPointD1 {
    ResampledPointD1 {
        id,
        instrument: "EURUSD".to_string(),
        open: 1.10,
        high: 1.11,
        low: 1.09,
        close: 1.105,
        volume: 1000,
        timestamp,
    }
}

// ============================================================
// SECTION 5: Into a SignalGroup - Shift to the Next Resampled Point
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_into_a_signal_group() {
    println!("============================================================");
    println!("SECTION 5: Into a SignalGroup - Shift to the Next Resampled Point");
    println!("============================================================");
    // Fri 2024-01-05, Mon 01-08, Tue 01-09, Wed 01-10.
    let timestamps = [1704412800, 1704672000, 1704758400, 1704844800];
    let values = [-0.5, 0.8, -0.3, 0.6];
    let items: Vec<IndicatorValue> = timestamps
        .iter()
        .zip(values)
        .map(|(&timestamp, value)| IndicatorValue { timestamp, value })
        .collect();
    let raw = detect_signals(&OscillatorBuffer { items: items.clone() });
    println!("raw detect_signals    longs {:?}, shorts {:?}", raw.0, raw.1);

    let points: Vec<ResampledPointD1> = (1..).zip(timestamps).map(|(id, t)| resampled_point(id, t)).collect();
    let output = IndicatorOutput::Oscillator { instrument: "EURUSD".to_string(), buffer: OscillatorBuffer { items } };
    let group = output.create_signal_group(7, &points);
    println!("SignalGroup           longs {:?}, shorts {:?}", group.long_signals, group.short_signals);
    println!("  indicator_id={}, instrument={}", group.indicator_id, group.instrument);
    println!();
    println!("  -> The value at T needs T's close, so entry is the NEXT resampled point.");
    println!("  -> Long at Mon 1704672000 moves to Tue 1704758400; short at Tue moves to Wed.");
    println!("  -> Long at Wed 1704844800 is the last point: nothing to enter, dropped.");
}
