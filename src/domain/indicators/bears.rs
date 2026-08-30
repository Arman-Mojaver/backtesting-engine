use crate::domain::indicators::moving_averages::ema;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorSeries};
use crate::validate_period;

/// Compute Bears Power from resampled daily points.
///
/// `Bears[i] = low[i] - EMA(close, period)[i]` where the EMA is seeded with
/// the SMA of the first `period` close values and recurses with
/// `k = 2 / (period + 1)` (MQL5 `Bears.mq5`, `MODE_EMA` on `PRICE_CLOSE`).
///
/// The EMA is valid from index `period - 1`; Bears starts one bar later at
/// index `period` (the MQL5 source sets `start = InpBearsPeriod`). Each value
/// is tagged with the timestamp of the resampled point at the evaluated index.
///
/// Insufficient data (fewer than `period + 1` points) yields an empty buffer,
/// not an error — consistent with the AO/AC reference and umbrella PRD
/// Decision 10. A non-positive period is an error.
pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<OscillatorSeries, IndicatorError> {
    validate_period!(period, InvalidBearsPeriod);
    let p = period as usize;
    let close: Vec<f64> = points.iter().map(|pt| pt.close).collect();
    let ema_close = ema(&close, p);

    // EMA entry `j` corresponds to original index `period - 1 + j`. Bears
    // starts at original index `period` (= EMA entry 1), so it consumes EMA
    // entries `1..ema_close.len()` — requiring at least two EMA entries
    // (i.e. `points.len() >= period + 1`).
    if ema_close.len() < 2 {
        return Ok(OscillatorSeries { items: Vec::new() });
    }

    let items = (1..ema_close.len())
        .map(|j| {
            // EMA entry j -> original index period - 1 + j; Bears evaluates
            // the bar at that same original index.
            let i = p - 1 + j;
            IndicatorValue {
                timestamp: points[i].timestamp,
                value: points[i].low - ema_close[j],
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
    use crate::indicator_values;

    /// Build a resampled point with explicit close and low (the two price
    /// components Bears Power consumes).
    fn rp(id: i32, ts: i64, close: f64, low: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: close,
            high: close,
            low,
            close,
            volume: 1000,
            timestamp: ts,
            high_low_order: HighLowOrder::HighFirst,
        }
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_close_yields_low_minus_constant_ema() {
        // All closes = 10.0, period = 3 -> EMA = 10.0 everywhere.
        // low = 9.0 for all bars -> Bears = 9.0 - 10.0 = -1.0.
        // 4 points (period + 1) -> 1 Bears value at points[3].ts.
        let points: Vec<ResampledPointD1> = (0..4)
            .map(|i| rp(i + 1, (i + 1) as i64, 10.0, 9.0))
            .collect();
        let result = compute(&points, 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, -1.0);
        assert_eq!(vals[0].timestamp, 4);
    }

    #[test]
    fn linear_close_ramp_yields_hand_computed_bears() {
        // close[i] = i, low[i] = i - 1, period = 3 -> k = 2/(3+1) = 0.5
        // EMA seed = SMA(0,1,2) = 1.0            (index 2)
        // EMA[3]   = (3 - 1.0)*0.5 + 1.0 = 2.0   (index 3)
        // Bears[3] = low[3] - EMA[3] = 2 - 2.0 = 0.0
        // 4 points -> 1 Bears value = 0.0 at points[3].ts.
        let points: Vec<ResampledPointD1> = (0..4)
            .map(|i| rp(i + 1, (i + 1) as i64, i as f64, (i - 1) as f64))
            .collect();
        let result = compute(&points, 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 4);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[], 13).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_period_plus_one_points_yields_empty_buffer() {
        // period = 3, 3 points -> EMA has 1 entry -> no Bears values.
        let points: Vec<ResampledPointD1> = (0..3)
            .map(|i| rp(i + 1, (i + 1) as i64, 10.0, 9.0))
            .collect();
        let result = compute(&points, 3).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_plus_one_points_yields_single_value() {
        // period = 3, 4 points -> EMA has 2 entries -> 1 Bears value.
        let points: Vec<ResampledPointD1> = (0..4)
            .map(|i| rp(i + 1, (i + 1) as i64, 10.0, 9.0))
            .collect();
        let result = compute(&points, 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, -1.0);
        assert_eq!(vals[0].timestamp, 4);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = vec![rp(1, 1, 10.0, 9.0)];
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidBearsPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidBearsPeriod { n: -3 }
        );
    }

    #[test]
    fn timestamps_track_evaluated_bar() {
        // period = 2, 4 points -> EMA has 3 entries -> 2 Bears values at
        // original indices 2 and 3 -> timestamps points[2].ts, points[3].ts.
        let points: Vec<ResampledPointD1> = (0..4)
            .map(|i| rp(i + 1, (i + 100) as i64, 10.0, 9.0))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].timestamp, 102);
        assert_eq!(vals[1].timestamp, 103);
    }

    #[test]
    fn output_length_is_points_minus_period() {
        // 10 points, period 3 -> 10 - 3 = 7 Bears values.
        let points: Vec<ResampledPointD1> = (0..10)
            .map(|i| rp(i + 1, (i + 1) as i64, 10.0, 9.0))
            .collect();
        let result = compute(&points, 3).unwrap();
        assert_eq!(indicator_values!(result).len(), 7);
    }

    // --- Level 2: real Bears Power output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from the real EURUSD development DB.
    ///
    /// These values are **not ground-truth** — they were produced by running
    /// `bears::compute` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `period = 13` (the MQL5 default) and capturing the output. They serve
    /// as a regression baseline. The user will later compare them against an
    /// external source of truth.
    #[test]
    fn level2_real_bears_output_matches_captured_baseline() {
        let points = match test_support::fetch_eurusd_points() {
            Some(p) => p,
            None => {
                eprintln!("skipping level2_baseline_test: development DB unavailable");
                return;
            }
        };
        let result = compute(&points, 13).unwrap();
        let vals = indicator_values!(result);

        // 2400 points -> 2387 Bears values (2400 - 13).
        assert_eq!(vals.len(), 2387);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1356048000);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1356048000, 0.005596153846153973),
            (1356307200, 0.005728131868131703),
            (1356480000, 0.005399827315541561),
            (1356566400, 0.005771280556178482),
            (1356652800, 0.0011582404767243037),
            (1356912000, 0.0011699204086208503),
            (1356998400, 0.003131360350246526),
            (1357084800, -0.0012888339855032793),
            (1357171200, -0.01259900055900287),
            (1357257600, -0.014124857622002551),
            (1357516800, -0.012031306533144992),
            (1357603200, -0.007182548456981408),
            (1357689600, -0.008165041534555328),
            (1357776000, -0.010100035601047441),
            (1357862400, 0.00798996948481645),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
