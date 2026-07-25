use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorSeries};

/// Compute the Rate of Change (ROC) from resampled daily points.
///
/// `ROC[i] = (close[i] - close[i - period]) / close[i] * 100` when
/// `close[i] != 0.0`, otherwise `0.0` (MQL5 `ROC.mq5`, `PRICE_CLOSE` baked in
/// via the `price[]` array which defaults to `PRICE_CLOSE`). No MA helpers are
/// needed — ROC is a pure price-momentum formula with a single subtraction,
/// division, and multiplication.
///
/// ROC entry `k` corresponds to original index `period + k`. The MQL5 source
/// sets `PLOT_DRAW_BEGIN = InpRocPeriod` (= `period`); warm-up bars produce no
/// `IndicatorValue` entries (per umbrella PRD Decision 10). Each value is
/// tagged with the timestamp of the resampled point at the evaluated index.
///
/// Insufficient data (fewer than `period + 1` points, i.e. `N <= period`)
/// yields an empty `items` vector (not an error), per umbrella PRD Decision 10.
/// A non-positive `period` is an error.
pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<OscillatorSeries, IndicatorError> {
    if period <= 0 {
        return Err(IndicatorError::InvalidRocPeriod { n: period });
    }
    let p = period as usize;
    let n = points.len();

    if n <= p {
        return Ok(OscillatorSeries { items: Vec::new() });
    }

    let closes: Vec<f64> = points.iter().map(|pt| pt.close).collect();

    let items = (0..(n - p))
        .map(|k| {
            let curr = closes[p + k];
            let prev = closes[k];
            let value = if curr == 0.0 {
                0.0
            } else {
                (curr - prev) / curr * 100.0
            };
            IndicatorValue {
                timestamp: points[p + k].timestamp,
                value,
            }
        })
        .collect();

    Ok(OscillatorSeries { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};

    /// Build a resampled point with an explicit close (the only component the
    /// ROC computation consumes).
    fn rp(id: i32, ts: i64, close: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume: 1000,
            timestamp: ts,
            high_low_order: HighLowOrder::HighFirst,
        }
    }

    fn values(buffer: &OscillatorSeries) -> &[IndicatorValue] {
        &buffer.items
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_close_yields_zero_roc() {
        // All bars: close=10, period=3, N=5.
        // ROC[k] = (10 - 10) / 10 * 100 = 0.0 for all k.
        // 5 points -> 2 ROC values = 0.0 at points[3].ts, points[4].ts.
        let points: Vec<ResampledPointD1> =
            (0..5).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 3).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[1].value, 0.0);
        assert_eq!(vals[1].timestamp, 5);
    }

    #[test]
    fn linearly_increasing_close_yields_hand_computed_roc() {
        // close[i] = i + 1, period=3, N=5.
        // ROC[0] = (4 - 1) / 4 * 100 = 75.0 at points[3].ts.
        // ROC[1] = (5 - 2) / 5 * 100 = 60.0 at points[4].ts.
        // 5 points -> 2 ROC values = 75.0, 60.0.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, (i + 1) as f64))
            .collect();
        let result = compute(&points, 3).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].value, 75.0);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[1].value, 60.0);
        assert_eq!(vals[1].timestamp, 5);
    }

    #[test]
    fn decreasing_close_yields_negative_roc() {
        // close = [10, 8, 6, 4, 2], period=2, N=5.
        // ROC[0] = (6 - 10) / 6 * 100 = -66.666... at points[2].ts.
        // ROC[1] = (4 - 8) / 4 * 100 = -100.0 at points[3].ts.
        // ROC[2] = (2 - 6) / 2 * 100 = -200.0 at points[4].ts.
        // 5 points -> 3 ROC values = -66.666..., -100.0, -200.0.
        let closes = [10.0, 8.0, 6.0, 4.0, 2.0];
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, closes[i as usize]))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 3);
        assert!((vals[0].value - (-66.66666666666667)).abs() < 1e-9);
        assert_eq!(vals[0].timestamp, 3);
        assert_eq!(vals[1].value, -100.0);
        assert_eq!(vals[1].timestamp, 4);
        assert_eq!(vals[2].value, -200.0);
        assert_eq!(vals[2].timestamp, 5);
    }

    #[test]
    fn zero_price_zero_division_guard_yields_zero_roc() {
        // close = [0, 1, 2, 0, 4], period=2, N=5.
        // ROC[k] = (closes[period+k] - closes[k]) / closes[period+k] * 100.
        // ROC[0] = (closes[2] - closes[0]) / closes[2] * 100
        //        = (2 - 0) / 2 * 100 = 100.0 at points[2].ts.
        // ROC[1]: closes[3] = 0.0 -> zero-division guard -> 0.0 at points[3].ts.
        // ROC[2] = (closes[4] - closes[2]) / closes[4] * 100
        //        = (4 - 2) / 4 * 100 = 50.0 at points[4].ts.
        // 5 points -> 3 ROC values = 100.0, 0.0, 50.0.
        let closes = [0.0, 1.0, 2.0, 0.0, 4.0];
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, closes[i as usize]))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].value, 100.0);
        assert_eq!(vals[0].timestamp, 3);
        assert_eq!(vals[1].value, 0.0);
        assert_eq!(vals[1].timestamp, 4);
        assert_eq!(vals[2].value, 50.0);
        assert_eq!(vals[2].timestamp, 5);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[], 12).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_period_plus_one_points_yields_empty_buffer() {
        // period=12, 11 points -> 11 <= 12 -> no ROC values.
        let points: Vec<ResampledPointD1> =
            (0..11).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 12).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_points_yields_empty_buffer() {
        // period=12, 12 points -> 12 <= 12 -> no ROC values.
        let points: Vec<ResampledPointD1> =
            (0..12).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 12).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_plus_one_points_yields_single_value() {
        // period=3, 4 points -> 1 ROC value.
        // close = [10, 10, 10, 10] -> ROC[0] = (10 - 10) / 10 * 100 = 0.0.
        let points: Vec<ResampledPointD1> =
            (0..4).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 3).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 4);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = vec![rp(1, 1, 10.0)];
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidRocPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidRocPeriod { n: -3 }
        );
    }

    #[test]
    fn timestamps_track_evaluated_bar() {
        // period=3, 5 points -> 2 ROC values at original indices 3 and 4 ->
        // timestamps points[3].ts, points[4].ts.
        let points: Vec<ResampledPointD1> =
            (0..5).map(|i| rp(i + 1, (i + 100) as i64, 10.0)).collect();
        let result = compute(&points, 3).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].timestamp, 103);
        assert_eq!(vals[1].timestamp, 104);
    }

    #[test]
    fn output_length_is_points_minus_period() {
        // 10 points, period 3 -> 10 - 3 = 7 ROC values.
        let points: Vec<ResampledPointD1> =
            (0..10).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 3).unwrap();
        assert_eq!(values(&result).len(), 7);
    }

    // --- Level 2: real ROC output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from the real EURUSD development DB.
    ///
    /// These values are **not ground-truth** — they were produced by running
    /// `roc::compute` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `period = 12` (the MQL5 default) and capturing the output. They serve
    /// as a regression baseline. The user will later compare them against an
    /// external source of truth.
    #[test]
    fn level2_real_roc_output_matches_captured_baseline() {
        let points = match test_support::fetch_eurusd_points() {
            Some(p) => p,
            None => {
                eprintln!("skipping level2_baseline_test: development DB unavailable");
                return;
            }
        };
        let result = compute(&points, 12).unwrap();
        let vals = values(&result);

        // 2400 points -> 2388 ROC values (2400 - 12).
        assert_eq!(vals.len(), 2388);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1355961600);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1355961600, 1.146813233273441),
            (1356048000, 0.8842242562581999),
            (1356307200, 1.7433836353985037),
            (1356480000, 2.225365536334274),
            (1356566400, 2.223028214776866),
            (1356652800, 1.5649357528793932),
            (1356912000, 0.9593525506956363),
            (1356998400, 0.9905113855800307),
            (1357084800, 0.20168627926726515),
            (1357171200, -0.9411963241949589),
            (1357257600, -1.1922526534891318),
            (1357516800, -0.5592295736446016),
            (1357603200, -1.2629776303114701),
            (1357689600, -1.0165466523670823),
            (1357776000, 0.6015014924473194),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
