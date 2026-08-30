use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorSeries};

/// Compute the Volume Rate of Change (VROC) from resampled daily points.
///
/// `VROC[i] = 100 * (volume[i] - volume[i - (period - 1)]) /
/// volume[i - (period - 1)]` when the previous volume is non-zero, otherwise
/// the previous VROC value is carried forward (MQL5 `VROC.mq5`,
/// `VOLUME_TICK` baked in via `point.volume as f64`). No MA helpers are needed
/// — VROC is a pure volume-momentum formula with a single subtraction,
/// division, and multiplication, plus a carry-forward zero-division guard.
///
/// VROC entry `k` corresponds to original index `(period - 1) + k`. The MQL5
/// source sets `PLOT_DRAW_BEGIN = ExtPeriodVROC - 1` (= `period - 1`); warm-up
/// bars produce no `IndicatorValue` entries (per umbrella PRD Decision 10).
/// Each value is tagged with the timestamp of the resampled point at the
/// evaluated index.
///
/// Insufficient data (fewer than `period` points, i.e. `N < period`) yields an
/// empty `items` vector (not an error), per umbrella PRD Decision 10. A
/// non-positive `period` is an error.
pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<OscillatorSeries, IndicatorError> {
    if period <= 0 {
        return Err(IndicatorError::InvalidVrocPeriod { n: period });
    }
    let lookback = (period - 1) as usize;
    let n = points.len();

    if n <= lookback {
        return Ok(OscillatorSeries { items: Vec::new() });
    }

    let volumes: Vec<f64> = points.iter().map(|pt| pt.volume as f64).collect();

    let mut items: Vec<IndicatorValue> = Vec::with_capacity(n - lookback);
    let mut carry = 0.0;
    for k in 0..(n - lookback) {
        let curr = volumes[lookback + k];
        let prev = volumes[k];
        let value = if prev != 0.0 {
            100.0 * (curr - prev) / prev
        } else {
            carry
        };
        carry = value;
        items.push(IndicatorValue {
            timestamp: points[lookback + k].timestamp,
            value,
        });
    }

    Ok(OscillatorSeries { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};
    use crate::indicator_values;

    /// Build a resampled point with an explicit volume (the only component the
    /// VROC computation consumes).
    fn rp(id: i32, ts: i64, volume: i32) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: 1.0,
            high: 1.0,
            low: 1.0,
            close: 1.0,
            volume,
            timestamp: ts,
            high_low_order: HighLowOrder::HighFirst,
        }
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_volume_yields_zero_vroc() {
        // All bars: volume=100, period=2, N=5.
        // VROC[k] = 100 * (100 - 100) / 100 = 0.0 for all k.
        // 5 points -> 4 VROC values = 0.0 at points[1].ts .. points[4].ts.
        let points: Vec<ResampledPointD1> =
            (0..5).map(|i| rp(i + 1, (i + 1) as i64, 100)).collect();
        let result = compute(&points, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 2);
        assert_eq!(vals[1].value, 0.0);
        assert_eq!(vals[1].timestamp, 3);
        assert_eq!(vals[2].value, 0.0);
        assert_eq!(vals[2].timestamp, 4);
        assert_eq!(vals[3].value, 0.0);
        assert_eq!(vals[3].timestamp, 5);
    }

    #[test]
    fn linearly_increasing_volume_yields_hand_computed_vroc() {
        // volume[i] = (i + 1) * 100, period=2, N=5.
        // VROC[0] = 100 * (200 - 100) / 100 = 100.0 at points[1].ts.
        // VROC[1] = 100 * (300 - 200) / 200 = 50.0 at points[2].ts.
        // VROC[2] = 100 * (400 - 300) / 300 = 33.333... at points[3].ts.
        // VROC[3] = 100 * (500 - 400) / 400 = 25.0 at points[4].ts.
        // 5 points -> 4 VROC values = 100.0, 50.0, 33.333..., 25.0.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, (i + 1) * 100))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 100.0);
        assert_eq!(vals[0].timestamp, 2);
        assert_eq!(vals[1].value, 50.0);
        assert_eq!(vals[1].timestamp, 3);
        assert!((vals[2].value - 33.333333333333336).abs() < 1e-9);
        assert_eq!(vals[2].timestamp, 4);
        assert_eq!(vals[3].value, 25.0);
        assert_eq!(vals[3].timestamp, 5);
    }

    #[test]
    fn decreasing_volume_yields_negative_vroc() {
        // volume = [500, 400, 300, 200, 100], period=2, N=5.
        // VROC[0] = 100 * (400 - 500) / 500 = -20.0 at points[1].ts.
        // VROC[1] = 100 * (300 - 400) / 400 = -25.0 at points[2].ts.
        // VROC[2] = 100 * (200 - 300) / 300 = -33.333... at points[3].ts.
        // VROC[3] = 100 * (100 - 200) / 200 = -50.0 at points[4].ts.
        // 5 points -> 4 VROC values = -20.0, -25.0, -33.333..., -50.0.
        let volumes = [500, 400, 300, 200, 100];
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, volumes[i as usize]))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, -20.0);
        assert_eq!(vals[0].timestamp, 2);
        assert_eq!(vals[1].value, -25.0);
        assert_eq!(vals[1].timestamp, 3);
        assert!((vals[2].value - (-33.333333333333336)).abs() < 1e-9);
        assert_eq!(vals[2].timestamp, 4);
        assert_eq!(vals[3].value, -50.0);
        assert_eq!(vals[3].timestamp, 5);
    }

    #[test]
    fn zero_volume_zero_division_carry_forward_guard() {
        // volume = [0, 100, 200, 0, 400], period=2, N=5.
        // VROC[0]: prev = vol[0] = 0 -> carry-forward -> 0.0 (first value) at points[1].ts.
        // VROC[1] = 100 * (200 - 100) / 100 = 100.0 at points[2].ts.
        // VROC[2] = 100 * (0 - 200) / 200 = -100.0 at points[3].ts.
        // VROC[3]: prev = vol[3] = 0 -> carry-forward -> VROC[2] = -100.0 at points[4].ts.
        // 5 points -> 4 VROC values = 0.0, 100.0, -100.0, -100.0.
        let volumes = [0, 100, 200, 0, 400];
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, volumes[i as usize]))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 2);
        assert_eq!(vals[1].value, 100.0);
        assert_eq!(vals[1].timestamp, 3);
        assert_eq!(vals[2].value, -100.0);
        assert_eq!(vals[2].timestamp, 4);
        assert_eq!(vals[3].value, -100.0);
        assert_eq!(vals[3].timestamp, 5);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[], 25).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_period_points_yields_empty_buffer() {
        // period=25, 24 points -> 24 < 25 -> no VROC values.
        let points: Vec<ResampledPointD1> =
            (0..24).map(|i| rp(i + 1, (i + 1) as i64, 100)).collect();
        let result = compute(&points, 25).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_minus_one_points_yields_empty_buffer() {
        // period=25, 24 points (= period - 1) -> 24 <= 24 (lookback) -> empty.
        let points: Vec<ResampledPointD1> =
            (0..24).map(|i| rp(i + 1, (i + 1) as i64, 100)).collect();
        let result = compute(&points, 25).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_points_yields_single_value() {
        // period=2, 2 points -> lookback=1 -> 1 VROC value.
        // volume = [100, 200] -> VROC[0] = 100 * (200 - 100) / 100 = 100.0.
        let points: Vec<ResampledPointD1> = vec![rp(1, 1, 100), rp(2, 2, 200)];
        let result = compute(&points, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 100.0);
        assert_eq!(vals[0].timestamp, 2);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = vec![rp(1, 1, 100)];
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidVrocPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidVrocPeriod { n: -3 }
        );
    }

    #[test]
    fn timestamps_track_evaluated_bar() {
        // period=3, 5 points -> lookback=2 -> 3 VROC values at original
        // indices 2, 3, 4 -> timestamps points[2].ts, points[3].ts,
        // points[4].ts.
        let points: Vec<ResampledPointD1> =
            (0..5).map(|i| rp(i + 1, (i + 100) as i64, 100)).collect();
        let result = compute(&points, 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].timestamp, 102);
        assert_eq!(vals[1].timestamp, 103);
        assert_eq!(vals[2].timestamp, 104);
    }

    #[test]
    fn output_length_is_points_minus_period_plus_one() {
        // 10 points, period 3 -> lookback=2 -> 10 - 2 = 8 VROC values.
        let points: Vec<ResampledPointD1> =
            (0..10).map(|i| rp(i + 1, (i + 1) as i64, 100)).collect();
        let result = compute(&points, 3).unwrap();
        assert_eq!(indicator_values!(result).len(), 8);
    }

    // --- Level 2: real VROC output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from the real EURUSD development DB.
    ///
    /// These values are **not ground-truth** — they were produced by running
    /// `vroc::compute` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `period = 25` (the MQL5 default) and capturing the output. They serve
    /// as a regression baseline. The user will later compare them against an
    /// external source of truth.
    #[test]
    fn level2_real_vroc_output_matches_captured_baseline() {
        let points = match test_support::fetch_eurusd_points() {
            Some(p) => p,
            None => {
                eprintln!("skipping level2_baseline_test: development DB unavailable");
                return;
            }
        };
        let result = compute(&points, 25).unwrap();
        let vals = indicator_values!(result);

        // 2400 points -> 2376 VROC values (2400 - 25 + 1).
        assert_eq!(vals.len(), 2376);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1357603200);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1357603200, -10.352604259889029),
            (1357689600, -9.832639852057328),
            (1357776000, -6.706926684085757),
            (1357862400, 2.9296324813611525),
            (1358121600, 1.2604957005563986),
            (1358208000, 11.266517270690056),
            (1358294400, 1.3997568943250018),
            (1358380800, 5.556197074672825),
            (1358467200, 2.0761771013227563),
            (1358726400, -20.859979804779535),
            (1358812800, 18.978701343586643),
            (1358899200, -15.570481661511268),
            (1358985600, 1.5318052103470496),
            (1359072000, -6.640015600624025),
            (1359331200, 55.371849650133136),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
