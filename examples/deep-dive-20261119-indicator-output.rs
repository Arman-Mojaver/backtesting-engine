//! Deep Dive: IndicatorOutput — routing to a detector, then shifting to the next resampled point
//! Source: Code input — src/domain/signal/mod.rs
//!
//! IndicatorOutput::create_signal_group is the only public entry point of the
//! module: it picks a detector by variant (oscillator / thresholds / crossover),
//! then moves every signal timestamp to the next resampled point, because a
//! value read at T's close can only be traded at T+1's open.

use std::env;

use backtesting_engine::domain::resampled_point::ResampledPointD1;
use backtesting_engine::domain::signal::{
    CrossoverBuffers, IndicatorOutput, IndicatorValue, OscillatorBuffer, ThresholdBuffer,
};

const DAY: i64 = 86_400;
const MON_1_JAN_2024: i64 = 1_704_067_200;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_routing(),
        "2" => section_2_threshold_levels(),
        "3" => section_3_crossover_diff(),
        "4" => section_4_shift_to_next_point(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-indicator-output <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Routing: the same values through all three variants");
    println!("  2    - Threshold Levels: crossing, touching, and passing through 30 / 70");
    println!("  3    - Crossover Diff: two buffers collapse into one oscillator");
    println!("  4    - Shift to Next Resampled Point: weekends, gaps, and the last point");
}

fn day(n: i64) -> i64 {
    MON_1_JAN_2024 + n * DAY
}

fn as_days(timestamps: &[i64]) -> Vec<i64> {
    timestamps.iter().map(|ts| (ts - MON_1_JAN_2024) / DAY).collect()
}

fn values(series: &[f64]) -> Vec<IndicatorValue> {
    (0..).zip(series).map(|(i, &value)| IndicatorValue { timestamp: day(i), value }).collect()
}

fn points_on(days: &[i64]) -> Vec<ResampledPointD1> {
    days.iter()
        .map(|&d| ResampledPointD1 {
            id: 0, instrument: "EURUSD".to_string(), open: 1.1, high: 1.1,
            low: 1.1, close: 1.1, volume: 0, timestamp: day(d),
        })
        .collect()
}

fn signals(output: IndicatorOutput, points: &[ResampledPointD1]) -> (Vec<i64>, Vec<i64>) {
    let sg = output.create_signal_group(1, points);
    (as_days(&sg.long_signals), as_days(&sg.short_signals))
}

// ============================================================
// SECTION 1: Routing - The Same Values Through All Three Variants
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_routing() {
    println!("============================================================");
    println!("SECTION 1: Routing - The Same Values Through All Three Variants");
    println!("============================================================");

    let series = [-5.0, 20.0, 40.0, 80.0, 60.0, 10.0, -3.0];
    let points = points_on(&(0..8).collect::<Vec<_>>());
    let instrument = || "EURUSD".to_string();
    println!("series by day 0..6: {series:?}");
    println!();

    let osc = IndicatorOutput::Oscillator { instrument: instrument(), buffer: OscillatorBuffer { items: values(&series) } };
    let thr = IndicatorOutput::Threshold {
        instrument: instrument(),
        buffer: ThresholdBuffer { items: values(&series), high_threshold: 70.0, low_threshold: 30.0 },
    };
    // Short buffer flat at 50: the crossover becomes "series vs 50".
    let cross = IndicatorOutput::Crossover {
        instrument: instrument(),
        buffers: CrossoverBuffers { upcross_long_buffer: values(&series), upcross_short_buffer: values(&[50.0; 7]) },
    };
    for (name, output) in [("Oscillator (0)", osc), ("Threshold (30/70)", thr), ("Crossover (vs 50)", cross)] {
        let (long, short) = signals(output, &points);
        println!("{name:<18} long days {long:?}  short days {short:?}");
    }
    println!();
    println!("  -> one match, three level choices: oscillator = 0/0, threshold =");
    println!("     low/high, crossover = 0/0 on (long - short)");
    println!("  -> every day printed is already shifted +1 (see section 4)");
}

// ============================================================
// SECTION 2: Threshold Levels - Crossing, Touching, Passing Through
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_threshold_levels() {
    println!("============================================================");
    println!("SECTION 2: Threshold Levels - Crossing, Touching, Passing Through");
    println!("============================================================");

    let points = points_on(&(0..8).collect::<Vec<_>>());
    let cases: [(&str, [f64; 4]); 4] = [
        ("clean cross above 30", [20.0, 25.0, 35.0, 40.0]),
        ("touch 30 and bounce", [20.0, 30.0, 25.0, 20.0]),
        ("pass through 30", [20.0, 30.0, 30.0, 40.0]),
        ("cross below 70", [80.0, 75.0, 65.0, 60.0]),
    ];
    for (label, series) in cases {
        let output = IndicatorOutput::Threshold {
            instrument: "EURUSD".to_string(),
            buffer: ThresholdBuffer { items: values(&series), high_threshold: 70.0, low_threshold: 30.0 },
        };
        let (long, short) = signals(output, &points);
        println!("{label:<22} {series:?} → long {long:?} short {short:?}");
    }
    println!();
    println!("  -> long = cross ABOVE the low level, short = cross BELOW the high level");
    println!("  -> values exactly on the level are filtered out before pairing,");
    println!("     so 20→30→30→40 is compared as 20→40: crossed day 3, entered day 4");
    println!("  -> a touch-and-bounce is never a signal");
}

// ============================================================
// SECTION 3: Crossover Diff - Two Buffers Collapse Into One Oscillator
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_crossover_diff() {
    println!("============================================================");
    println!("SECTION 3: Crossover Diff - Two Buffers Collapse Into One Oscillator");
    println!("============================================================");

    let fast = [1.10, 1.12, 1.15, 1.13, 1.09];
    let slow = [1.12, 1.12, 1.13, 1.13, 1.11];
    let diff: Vec<String> = fast.iter().zip(slow).map(|(f, s)| format!("{:+.2}", f - s)).collect();
    println!("fast (upcross_long_buffer):  {fast:?}");
    println!("slow (upcross_short_buffer): {slow:?}");
    println!("diff = fast - slow:          {diff:?}");

    let points = points_on(&(0..6).collect::<Vec<_>>());
    let full = IndicatorOutput::Crossover {
        instrument: "EURUSD".to_string(),
        buffers: CrossoverBuffers { upcross_long_buffer: values(&fast), upcross_short_buffer: values(&slow) },
    };
    let (long, short) = signals(full, &points);
    println!("→ long days {long:?}  short days {short:?}");

    let truncated = IndicatorOutput::Crossover {
        instrument: "EURUSD".to_string(),
        buffers: CrossoverBuffers { upcross_long_buffer: values(&fast), upcross_short_buffer: values(&slow[..3]) },
    };
    let (long, short) = signals(truncated, &points);
    println!("slow cut to 3 values → long days {long:?}  short days {short:?}");
    println!();
    println!("  -> zero diffs (days 1, 3) are dropped: crosses on days 2 and 4, entered 3 and 5");
    println!("  -> buffers are zipped by POSITION, not timestamp; the long buffer's");
    println!("     timestamp is kept, and the shorter buffer silently truncates");
}

// ============================================================
// SECTION 4: Shift to Next Resampled Point - Weekends, Gaps, Last Point
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_shift_to_next_point() {
    println!("============================================================");
    println!("SECTION 4: Shift to Next Resampled Point - Weekends, Gaps, Last Point");
    println!("============================================================");

    // Mon..Fri, then the next Mon/Tue: days 5 and 6 (Sat/Sun) have no point.
    let points = points_on(&[0, 1, 2, 3, 4, 7, 8]);
    println!("resampled point days: [0, 1, 2, 3, 4, 7, 8]  (weekend 5-6 missing)");

    let cases: [(&str, Vec<IndicatorValue>); 3] = [
        ("cross on Thu (day 3)", vec![iv(2, -1.0), iv(3, 1.0)]),
        ("cross on Fri (day 4)", vec![iv(3, -1.0), iv(4, 1.0)]),
        ("cross on last point (8)", vec![iv(7, -1.0), iv(8, 1.0)]),
    ];
    for (label, items) in cases {
        let output = IndicatorOutput::Oscillator { instrument: "EURUSD".to_string(), buffer: OscillatorBuffer { items } };
        let (long, _) = signals(output, &points);
        println!("{label:<24} → long entry days {long:?}");
    }
    // An indicator stamped on a day with no resampled point still finds the next one.
    let off_grid = IndicatorOutput::Oscillator {
        instrument: "EURUSD".to_string(),
        buffer: OscillatorBuffer { items: vec![iv(4, -1.0), iv(5, 1.0)] },
    };
    println!("{:<24} → long entry days {:?}", "cross on Sat (day 5)", signals(off_grid, &points).0);
    println!();
    println!("  -> partition_point(ts <= signal) finds the first point strictly AFTER");
    println!("     the signal: Fri → Mon (day 7), not Fri + 1 day");
    println!("  -> no next point → filter_map drops the signal, it never errors");
    println!("  -> binary search assumes resampled_points are sorted by timestamp");
}

fn iv(d: i64, value: f64) -> IndicatorValue {
    IndicatorValue { timestamp: day(d), value }
}
