//! Deep Dive: crossover — two-line cross detection via a difference buffer
//! Source: Code input — src/domain/signal/crossover.rs
//!
//! Crossover indicators (today: MACD) emit two lines. detect_signals zips the
//! long line with the short line into one buffer of (long - short) and hands it
//! to oscillator::detect_signals: "long crosses above short" is exactly
//! "the difference crosses up through zero".

use std::env;

use backtesting_engine::domain::resampled_point::ResampledPointD1;
use backtesting_engine::domain::signal::crossover::detect_signals;
use backtesting_engine::domain::signal::{CrossoverBuffers, IndicatorOutput, IndicatorValue};

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_difference_buffer(),
        "2" => section_2_relative_not_absolute(),
        "3" => section_3_touching_lines(),
        "4" => section_4_alignment_trust(),
        "5" => section_5_into_a_signal_group(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-crossover <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Difference Buffer: long - short, then a zero-line cross");
    println!("  2    - Relative Not Absolute: only the gap's sign matters");
    println!("  3    - Touching Lines: equal values are skipped, float noise is not");
    println!("  4    - Alignment Trust: zip truncates, short timestamps are ignored");
    println!("  5    - Into a SignalGroup: MACD-shaped buffers through create_signal_group");
}

fn line(points: &[(i64, f64)]) -> Vec<IndicatorValue> {
    points
        .iter()
        .map(|&(timestamp, value)| IndicatorValue { timestamp, value })
        .collect()
}

// Index-as-timestamp keeps output readable; real buffers carry resampled-point timestamps.
fn buffers(long: &[f64], short: &[f64]) -> CrossoverBuffers {
    let indexed = |values: &[f64]| line(&(0..).zip(values.iter().copied()).collect::<Vec<_>>());
    CrossoverBuffers {
        upcross_long_buffer: indexed(long),
        upcross_short_buffer: indexed(short),
    }
}

fn show(label: &str, long: &[f64], short: &[f64]) {
    let diffs: Vec<f64> = long.iter().zip(short).map(|(l, s)| l - s).collect();
    let (longs, shorts) = detect_signals(&buffers(long, short));
    println!("{label}");
    println!("  long {long:?}  short {short:?}");
    println!("  diff {diffs:?} -> longs {longs:?}, shorts {shorts:?}");
}

// ============================================================
// SECTION 1: Difference Buffer - Long Minus Short, Then a Zero-Line Cross
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_difference_buffer() {
    println!("============================================================");
    println!("SECTION 1: Difference Buffer - Long Minus Short, Then a Zero-Line Cross");
    println!("============================================================");
    show("up cross (long overtakes short)", &[0.8, 1.2], &[1.0, 0.9]);
    show("down cross (long falls below short)", &[1.5, 0.7], &[1.0, 1.2]);
    show("up, down, up", &[0.0, 2.0, -1.0, 1.0], &[1.0, 1.5, 1.2, 0.5]);
    println!();
    println!("  -> diff < 0 then diff > 0: long signal at the SECOND point.");
    println!("  -> diff > 0 then diff < 0: short signal at the second point.");
    println!("  -> Every flip signals; there is no pairing or alternation rule.");
}

// ============================================================
// SECTION 2: Relative Not Absolute - Only the Gap's Sign Matters
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_relative_not_absolute() {
    println!("============================================================");
    println!("SECTION 2: Relative Not Absolute - Only the Gap's Sign Matters");
    println!("============================================================");
    show("both lines rise, long stays above", &[2.0, 3.0], &[1.0, 1.5]);
    show("both negative, long crosses up", &[-2.0, 0.5], &[-1.0, -0.2]);
    show("both negative, long crosses down", &[-0.5, -1.2], &[-1.0, -0.8]);
    show("gap shrinks but never flips", &[-2.0, -1.2], &[-1.0, -0.5]);
    println!();
    println!("  -> Where the lines sit (above/below zero) is irrelevant.");
    println!("  -> A narrowing gap is not a signal; only a sign change is.");
}

// ============================================================
// SECTION 3: Touching Lines - Equal Values Are Skipped, Float Noise Is Not
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_touching_lines() {
    println!("============================================================");
    println!("SECTION 3: Touching Lines - Equal Values Are Skipped, Float Noise Is Not");
    println!("============================================================");
    show("touch then cross", &[0.2, 0.5, 0.9], &[0.5, 0.5, 0.5]);
    show("touch and return", &[0.8, 0.5, 0.8], &[0.5, 0.5, 0.5]);
    show("touch via 0.1 + 0.2", &[0.4, 0.3, 0.4], &[0.35, 0.1 + 0.2, 0.35]);
    println!();
    println!("  -> level_crosses drops items whose diff == 0.0 before windowing,");
    println!("     so a touch is invisible: the cross lands on the next real side.");
    println!("  -> 0.3 - (0.1 + 0.2) = -5.55e-17, not 0.0: a mathematical touch can");
    println!("     become a phantom short + long pair. No epsilon is applied.");
}

// ============================================================
// SECTION 4: Alignment Trust - Zip Truncates, Short Timestamps Are Ignored
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_alignment_trust() {
    println!("============================================================");
    println!("SECTION 4: Alignment Trust - Zip Truncates, Short Timestamps Are Ignored");
    println!("============================================================");
    show("long longer than short (tail dropped)", &[-1.0, 1.0, -1.0], &[0.0, 0.0]);

    let misaligned = CrossoverBuffers {
        upcross_long_buffer: line(&[(100, 0.8), (200, 1.2)]),
        upcross_short_buffer: line(&[(999, 1.0), (888, 0.9)]),
    };
    println!("misaligned timestamps: long @[100,200], short @[999,888]");
    println!("  -> {:?}", detect_signals(&misaligned));
    show("empty", &[], &[]);
    show("single pair", &[1.0], &[2.0]);
    println!();
    println!("  -> Pairing is by position, not timestamp; the diff takes long's");
    println!("     timestamp. Alignment is the producer's contract (macd::compute");
    println!("     drops the first SIGNAL_N - 1 MACD values so both buffers line up).");
    println!("  -> Fewer than two non-zero diffs: no window, no signal.");
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
// SECTION 5: Into a SignalGroup - MACD-Shaped Buffers Through create_signal_group
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_into_a_signal_group() {
    println!("============================================================");
    println!("SECTION 5: Into a SignalGroup - MACD-Shaped Buffers Through create_signal_group");
    println!("============================================================");
    // Fri 2024-01-05, Mon 01-08, Tue 01-09, Wed 01-10.
    let timestamps = [1704412800, 1704672000, 1704758400, 1704844800];
    let macd_line = [-0.0012, 0.0004, 0.0009, -0.0003];
    let signal_line = [-0.0008, -0.0002, 0.0003, 0.0002];
    let crossover = CrossoverBuffers {
        upcross_long_buffer: line(&timestamps.into_iter().zip(macd_line).collect::<Vec<_>>()),
        upcross_short_buffer: line(&timestamps.into_iter().zip(signal_line).collect::<Vec<_>>()),
    };
    let raw = detect_signals(&crossover);
    println!("raw detect_signals    longs {:?}, shorts {:?}", raw.0, raw.1);

    let points: Vec<ResampledPointD1> = (1..).zip(timestamps).map(|(id, t)| resampled_point(id, t)).collect();
    let output = IndicatorOutput::Crossover { instrument: "EURUSD".to_string(), buffers: crossover };
    let group = output.create_signal_group(42, &points);
    println!("SignalGroup           longs {:?}, shorts {:?}", group.long_signals, group.short_signals);
    println!("  indicator_id={}, instrument={}", group.indicator_id, group.instrument);
    println!();
    println!("  -> MACD crosses above its signal line on Mon; entry shifts to Tue.");
    println!("  -> MACD crosses below on Wed, the last point: nothing to enter, dropped.");
}
