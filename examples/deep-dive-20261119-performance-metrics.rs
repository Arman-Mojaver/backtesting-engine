//! Deep Dive: performance_metrics + PerformanceMetrics::annual_roi
//! Source: Code input — src/domain/performance_metrics.rs
//!
//! A strategy is scored by replaying its operation points as a compounding
//! equity curve that starts at 1.0. Two numbers come out of one pass:
//! global_roi (where the curve ends) and max_draw_down (the worst fall from a
//! running peak). annual_roi then rescales global_roi to a per-year rate.
//! The only production caller is application/strategy_generator.rs, which
//! chains long + short ops into one Vec and stores annual_roi + max_draw_down.

use std::env;

use backtesting_engine::domain::operation_point::OperationPoint;
use backtesting_engine::domain::performance_metrics::{PerformanceMetrics, performance_metrics};

// Mirrors fixtures::ONE_YEAR_SECS (cfg(test), not reachable from examples).
const YEAR: i64 = 365 * 86_400 + 86_400 / 4;
// 2024-01-01T00:00:00Z
const T0: i64 = 1_704_067_200;
const HOUR: i64 = 3_600;

fn op(timestamp: i64, result: i32, sl: i32, risk: f64) -> OperationPoint {
    OperationPoint {
        id: 1,
        instrument: "EURUSD".into(),
        money_management_strategy_id: 1,
        timestamp,
        result,
        tp: 30,
        sl,
        risk,
    }
}

// Spaces ops one hour apart so they are already chronological.
fn series(trades: &[(i32, i32)], risk: f64) -> Vec<OperationPoint> {
    trades
        .iter()
        .zip(0_i64..)
        .map(|(&(result, sl), i)| op(T0 + i * HOUR, result, sl, risk))
        .collect()
}

fn metrics(ops: &[OperationPoint]) -> PerformanceMetrics {
    performance_metrics(ops.iter().collect())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_equity_curve_trace(),
        "2" => section_2_risk_over_sl(),
        "3" => section_3_sized_on_peak(),
        "4" => section_4_draw_down_from_peak(),
        "5" => section_5_chronological_sort(),
        "6" => section_6_rounding_and_empty(),
        "7" => section_7_annual_roi_guards(),
        "8" => section_8_annual_roi_double_rounding(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-performance-metrics <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Equity Curve Trace: cumsum and cummax step by step");
    println!("  2    - Risk over SL: why a full stop-loss always costs `risk`");
    println!("  3    - Sized on Peak: losses keep peak-sized stakes, ROI below -100%");
    println!("  4    - Draw-Down from Peak: deeper earlier dip vs new-peak dip");
    println!("  5    - Chronological Sort: scrambled input, stable ties, long+short");
    println!("  6    - Rounding and Empty Input: 2 decimals, the 1.0 baseline");
    println!("  7    - Annual ROI Guards: compounding, None span, total loss");
    println!("  8    - Annual ROI Double Rounding: rounded global_roi amplified");
}

// ============================================================
// SECTION 1: Equity Curve Trace - cumsum and cummax Step by Step
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_equity_curve_trace() {
    println!("============================================================");
    println!("SECTION 1: Equity Curve Trace - cumsum and cummax Step by Step");
    println!("============================================================");

    // +30 pips, -20, -20, +30 — each with sl=20 and 2% risk.
    let ops = series(&[(30, 20), (-20, 20), (-20, 20), (30, 20)], 0.02);

    // Local mirror of the loop body, only to expose the intermediate state.
    let (mut cumsum, mut cummax, mut max_dd) = (1.0_f64, 1.0_f64, 0.0_f64);
    println!(" # result  contribution    cumsum    cummax   draw_down");
    for (i, p) in ops.iter().enumerate() {
        let contribution = cummax * f64::from(p.result) * (p.risk / f64::from(p.sl));
        cumsum += contribution;
        cummax = cummax.max(cumsum);
        let dd = (cummax - cumsum) / cummax;
        max_dd = max_dd.max(dd);
        println!(
            "{:>2} {:>6} {:>13.6} {:>9.6} {:>9.6} {:>10.4}%",
            i + 1, p.result, contribution, cumsum, cummax, dd * 100.0
        );
    }
    println!();
    println!("mirror:  roi={:.4}%  max_dd={:.4}%", (cumsum - 1.0) * 100.0, max_dd * 100.0);
    println!("library: {:?}", metrics(&ops));
    println!();
    println!("  -> cumsum is equity (starts at 1.0 = 100%); cummax is its running peak");
    println!("  -> draw_down is measured against cummax: only a NEW peak resets it to 0");
    println!("     (row 4 wins but stays below 1.03, so draw_down is still 1%)");
}

// ============================================================
// SECTION 2: Risk over SL - Why a Full Stop-Loss Always Costs `risk`
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_risk_over_sl() {
    println!("============================================================");
    println!("SECTION 2: Risk over SL - Why a Full Stop-Loss Always Costs `risk`");
    println!("============================================================");

    // risk / sl is "equity fraction per pip": position size is chosen so that
    // hitting the stop loses exactly `risk`, whatever the stop distance.
    // That makes trades with very different stops comparable.
    println!("   sl  result  risk/sl     contribution (from 1.0)");
    for (result, sl) in [(-10, 10), (-20, 20), (-250, 250), (30, 20), (-5, 20)] {
        let ops = series(&[(result, sl)], 0.02);
        let per_pip = 0.02 / f64::from(sl);
        println!(
            "{:>5} {:>7} {:>8.5} {:>12.4}  -> roi {:>6}%",
            sl, result, per_pip, f64::from(result) * per_pip, metrics(&ops).global_roi
        );
    }
    println!();
    println!("  -> result == -sl always gives -2.00% (the configured risk)");
    println!("  -> result = +30 on sl 20 is a 1.5R win = +3%");
    println!("  -> result = -5 on sl 20 (early exit) loses only a quarter of risk");
    println!("  -> sl <= 0 is rejected by the PG adapter; a debug_assert backs it up,");
    println!("     so the division never produces inf/NaN");
}

// ============================================================
// SECTION 3: Sized on Peak - Losses Keep Peak-Sized Stakes
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_sized_on_peak() {
    println!("============================================================");
    println!("SECTION 3: Sized on Peak - Losses Keep Peak-Sized Stakes");
    println!("============================================================");

    // The stake multiplies cummax, not cumsum: after a loss the next trade is
    // still sized on the previous peak. Gains compound (peak rises), losses
    // do NOT shrink the stake — it's a conservative "fixed-on-high-water" model.
    let ops = series(&[(-20, 20), (-20, 20), (-20, 20)], 0.5);
    let on_peak = metrics(&ops);

    // What sizing on current equity would give instead: 1 * 0.5^3.
    let on_equity = (0.5_f64.powi(3) - 1.0) * 100.0;

    println!("Three full stop-losses at 50% risk each:");
    println!("  sized on cummax (actual): roi={}%  dd={}%", on_peak.global_roi, on_peak.max_draw_down);
    println!("  sized on cumsum (NOT used): roi={on_equity:.1}%");
    println!();
    println!("  -> cumsum = 1 - 0.5 - 0.5 - 0.5 = -0.5, so ROI and DD exceed 100%");
    println!("  -> this is intentional: a strategy can be scored worse than ruin,");
    println!("     which annual_roi then has to guard against (see section 7)");

    println!();
    let wins = series(&[(30, 20), (30, 20), (30, 20)], 0.02);
    println!("Three +1.5R wins at 2%: roi={}% (1.03^3 - 1 = 9.27%, compounding)", metrics(&wins).global_roi);
}

// ============================================================
// SECTION 4: Draw-Down from Peak - Deeper Earlier Dip vs New-Peak Dip
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_draw_down_from_peak() {
    println!("============================================================");
    println!("SECTION 4: Draw-Down from Peak - Deeper Earlier Dip vs New-Peak Dip");
    println!("============================================================");

    let base = [(-20, 20), (-20, 20), (30, 20), (25, 20), (-200, 200)];
    let cases: [(&str, Vec<(i32, i32)>); 3] = [
        ("earlier dip is deepest", [&base[..], &[(300, 250)]].concat()),
        ("new-peak dip deepens", [&base[..], &[(-250, 250), (-250, 250)]].concat()),
        (
            "partial recovery, same peak",
            [&base[..], &[(-250, 250), (-250, 250), (25, 250), (-250, 250)]].concat(),
        ),
    ];
    for (label, trades) in cases {
        let m = metrics(&series(&trades, 0.02));
        println!("{label:<28} trades={}  max_dd={:>4}%  roi={:>5}%", trades.len(), m.max_draw_down, m.global_roi);
    }
    println!();
    println!("  -> max_dd keeps the worst peak-to-trough ratio seen so far");
    println!("  -> a small win (+25 on sl 250 = +0.2%) does not make a new peak,");
    println!("     so the next loss is still measured from the old cummax");
    println!("  -> the ratio is relative to the peak, so a 2% loss from a higher");
    println!("     peak is still ~2%, not more");
}

// ============================================================
// SECTION 5: Chronological Sort - Scrambled Input, Stable Ties, Long+Short
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_chronological_sort() {
    println!("============================================================");
    println!("SECTION 5: Chronological Sort - Scrambled Input, Stable Ties, Long+Short");
    println!("============================================================");

    // strategy_generator passes long ops then short ops chained together, so
    // the input is NOT chronological — the sort_by_key inside fixes that.
    let longs = [op(T0, -20, 20, 0.02), op(T0 + 2 * HOUR, -20, 20, 0.02)];
    let shorts = [op(T0 + HOUR, 30, 20, 0.02)];
    let chained: Vec<&OperationPoint> = longs.iter().chain(shorts.iter()).collect();
    let order: Vec<i32> = chained.iter().map(|p| p.result).collect();
    println!("chained input (long, long, short): {order:?}  -> time order [-20, +30, -20]");
    println!("  actual (sorted):     {:?}", performance_metrics(chained));

    // Same trades re-stamped so input order == time order: what skipping the sort would give.
    let input_order = series(&[(-20, 20), (-20, 20), (30, 20)], 0.02);
    println!("  if input order used: {:?}", metrics(&input_order));
    println!();

    // Equal timestamps: sort_by_key is stable, so input order decides.
    let loss_first = [op(T0, -20, 20, 0.02), op(T0, 30, 20, 0.02)];
    let win_first = [op(T0, 30, 20, 0.02), op(T0, -20, 20, 0.02)];
    println!("tie, [-20, +30]: roi={}%", metrics(&loss_first).global_roi);
    println!("tie, [+30, -20]: roi={}%", metrics(&win_first).global_roi);
    println!();
    println!("  -> path-dependent: same trades, different order, different ROI");
    println!("  -> ties keep caller order (long before short at the same bar)");
}

// ============================================================
// SECTION 6: Rounding and Empty Input - 2 Decimals, the 1.0 Baseline
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_rounding_and_empty() {
    println!("============================================================");
    println!("SECTION 6: Rounding and Empty Input - 2 Decimals, the 1.0 Baseline");
    println!("============================================================");

    // rounded_percent(fraction) = round_to(fraction * 100, METRIC_DECIMALS=2)
    let empty = performance_metrics(Vec::new());
    println!("empty input:                 {empty:?}");
    println!("  -> loop never runs: cumsum=1.0 -> roi 0, dd 0 (not NaN)");
    println!();

    let tiny = series(&[(1, 7)], 0.01);
    println!("+1 pip on sl 7 at 1% risk:   raw roi = {:.6}%", 100.0 * 0.01 / 7.0);
    println!("                             stored  = {}%", metrics(&tiny).global_roi);

    let below_half_cent = series(&[(1, 300)], 0.01);
    println!("+1 pip on sl 300 at 1%:      raw roi = {:.6}%", 100.0 * 0.01 / 300.0);
    println!("                             stored  = {}%", metrics(&below_half_cent).global_roi);
    println!();
    println!("  -> both fields are percent with 2 decimals, matching the DB columns");
    println!("  -> sub-0.005% effects vanish entirely after rounding");
}

// ============================================================
// SECTION 7: Annual ROI Guards - Compounding, None Span, Total Loss
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_7_annual_roi_guards() {
    println!("============================================================");
    println!("SECTION 7: Annual ROI Guards - Compounding, None Span, Total Loss");
    println!("============================================================");

    let m = |global_roi| PerformanceMetrics { global_roi, max_draw_down: 0.0 };

    // annual = (1 + g/100)^(1/years) - 1 — geometric, not g / years.
    println!("global 21% over 2y:    annual={}%  (linear would say 10.5)", m(21.0).annual_roi(T0, T0 + 2 * YEAR));
    println!("global 10% over 1y:    annual={}%", m(10.0).annual_roi(T0, T0 + YEAR));
    println!("global 4.88% over 0.5y: annual={}%  (short spans extrapolate up)", m(4.88).annual_roi(T0, T0 + YEAR / 2));
    println!("global -10% over 1y:   annual={}%", m(-10.0).annual_roi(T0, T0 + YEAR));
    println!();

    // Guard 1: difference_in_years returns None for spans <= 0.
    println!("zero span:             annual={}%", m(10.0).annual_roi(T0, T0));
    println!("negative span:         annual={}%", m(10.0).annual_roi(T0 + YEAR, T0));

    // Guard 2: accumulated <= 0 — powf on a non-positive base is NaN/0.
    println!("global -99%:           annual={}%", m(-99.0).annual_roi(T0, T0 + YEAR));
    println!("global -100%:          annual={}%  (guarded)", m(-100.0).annual_roi(T0, T0 + YEAR));
    println!("global -150%:          annual={}%  (guarded, raw powf = {})", m(-150.0).annual_roi(T0, T0 + YEAR), (-0.5_f64).powf(1.0 / 1.5));
    println!();
    println!("  -> both guards return 0.0, so a ruined strategy looks like a flat one");
    println!("     in annual_roi; max_draw_down (>= 100%) is what tells them apart");
}

// ============================================================
// SECTION 8: Annual ROI Double Rounding - Rounded global_roi Amplified
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_8_annual_roi_double_rounding() {
    println!("============================================================");
    println!("SECTION 8: Annual ROI Double Rounding - Rounded global_roi Amplified");
    println!("============================================================");

    // annual_roi reads self.global_roi, which is ALREADY rounded to 2 dp, then
    // rounds again. Over short spans the exponent 1/years > 1 magnifies the
    // first rounding error.
    let ops = series(&[(1, 7)], 0.01);
    let m = metrics(&ops);
    let raw_global = 0.01_f64 / 7.0;
    println!("raw global fraction: {raw_global:.8}   stored global_roi: {}%", m.global_roi);
    println!();
    println!("  span    from stored   from raw (unrounded)");
    for (label, span) in [("1y", YEAR), ("0.25y", YEAR / 4), ("1 week", 7 * 86_400)] {
        let years = span as f64 / YEAR as f64;
        let from_raw = ((1.0 + raw_global).powf(1.0 / years) - 1.0) * 100.0;
        println!("{label:>7} {:>12}% {:>18.4}%", m.annual_roi(T0, T0 + span), from_raw);
    }
    println!();
    println!("  -> the gap grows as the span shrinks below a year (exponent > 1);");
    println!("     over multi-year spans the exponent is < 1 and the gap shrinks");
}
