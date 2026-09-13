use crate::domain::indicators::moving_averages::sma;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorSeries};

/// Fast SMA period (fixed per the MQL5 `Accelerator.mq5` source).
const FAST_PERIOD: usize = 5;
/// Slow SMA period (fixed per the MQL5 `Accelerator.mq5` source).
const SLOW_PERIOD: usize = 34;
/// Number of warm-up bars before the first valid AC value
/// (`FAST_PERIOD - 1 + SLOW_PERIOD - 1` = 4 + 33).
const DATA_LIMIT: usize = 37;

/// Compute the inlined Awesome Oscillator series as a plain `Vec<f64>`.
///
/// `AO[k] = SMA(median, 5)[k + offset] − SMA(median, 34)[k]` where
/// `offset = SLOW_PERIOD − FAST_PERIOD` and `median = (high + low) / 2`.
/// The returned series has length `N − SLOW_PERIOD + 1` (= `N − 33`); entry
/// `k` corresponds to the resampled point at ending index `k + SLOW_PERIOD − 1`.
///
/// Inlined (rather than calling `ao::compute`) per umbrella PRD Decision 6 so
/// AC stays self-contained with no cross-module coupling to `ao.rs`.
fn ao_series(points: &[ResampledPointD1]) -> Vec<f64> {
    let medians: Vec<f64> = points
        .iter()
        .map(|p| f64::midpoint(p.high, p.low))
        .collect();
    let fast = sma(&medians, FAST_PERIOD);
    let slow = sma(&medians, SLOW_PERIOD);
    let offset = SLOW_PERIOD - FAST_PERIOD;
    slow.iter()
        .zip(fast.iter().skip(offset))
        .map(|(slow_val, fast_val)| fast_val - slow_val)
        .collect()
}

/// Compute the Accelerator/Decelerator oscillator from resampled daily points.
///
/// `AC[k] = AO[k + 4] − SMA(AO, 5)[k]` where `AO` is the inlined Awesome
/// Oscillator series. The first valid AC value is at index `DATA_LIMIT` (37);
/// warm-up bars produce no `IndicatorValue` entries. Each value is tagged
/// with the timestamp of the resampled point at index `37 + k` (the newest
/// contributing point).
///
/// Insufficient data (fewer than 38 points) yields an empty buffer, not an
/// error — consistent with the AO reference and umbrella PRD Decision 10.
pub fn compute(points: &[ResampledPointD1]) -> Result<OscillatorSeries, IndicatorError> {
    let ao = ao_series(points);
    let sma_ao = sma(&ao, FAST_PERIOD);

    // AC[k] = AO[k + 4] − SMA(AO, 5)[k]; AO[k+4] ends at index k + 37
    // (= DATA_LIMIT + k), the newest contributing point.
    let items = sma_ao
        .iter()
        .zip(ao.iter().skip(FAST_PERIOD - 1))
        .zip(points.iter().skip(DATA_LIMIT))
        .map(|((sma_ao_val, ao_val), point)| IndicatorValue {
            timestamp: point.timestamp,
            value: ao_val - sma_ao_val,
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

    /// Build a resampled point with explicit high/low so the median
    /// `(high + low) / 2` is controlled.
    fn rp(id: i32, ts: i64, high: f64, low: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: high,
            high,
            low,
            close: high,
            volume: 1000,
            timestamp: ts,
            high_low_order: HighLowOrder::HighFirst,
        }
    }

    /// Point whose median price `(high + low) / 2` equals `median`.
    fn rp_median(id: i32, ts: i64, median: f64) -> ResampledPointD1 {
        rp(id, ts, median + 0.5, median - 0.5)
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_median_yields_zero_ac() {
        // All medians equal 5.0 -> AO = 0 everywhere -> SMA(AO,5) = 0 -> AC = 0.
        // 38 points -> 1 AC value = 0.0 at points[37].ts.
        let points: Vec<ResampledPointD1> = (0..38)
            .map(|i| rp_median(i + 1, (i + 1) as i64, 5.0))
            .collect();
        let result = compute(&points).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 38);
    }

    #[test]
    fn quadratic_median_yields_constant_ac_58() {
        // median[i] = i^2 -> AO[k] = 29k + 594.5 (linear) -> SMA(AO,5) linear
        // -> AC[k] = AO[k+4] - SMA(AO,5)[k] = 58.0 constant.
        // 40 points -> 3 AC values at points[37].ts, points[38].ts, points[39].ts.
        let points: Vec<ResampledPointD1> = (0..40)
            .map(|i| rp_median(i + 1, (i + 100) as i64, (i * i) as f64))
            .collect();
        let result = compute(&points).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].value, 58.0);
        assert_eq!(vals[1].value, 58.0);
        assert_eq!(vals[2].value, 58.0);
        assert_eq!(vals[0].timestamp, 137);
        assert_eq!(vals[1].timestamp, 138);
        assert_eq!(vals[2].timestamp, 139);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[]).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_38_points_yields_empty_buffer() {
        // 37 points -> AO has 4 values -> SMA(AO,5) has 0 windows -> empty.
        let points: Vec<ResampledPointD1> = (0..37)
            .map(|i| rp_median(i + 1, (i + 1) as i64, 1.0))
            .collect();
        let result = compute(&points).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_38_points_yields_single_value() {
        let points: Vec<ResampledPointD1> = (0..38)
            .map(|i| rp_median(i + 1, (i + 1) as i64, 1.0))
            .collect();
        let result = compute(&points).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        // constant median -> AC = 0
        assert_eq!(vals[0].value, 0.0);
        // timestamp of the 38th point (index 37)
        assert_eq!(vals[0].timestamp, 38);
    }

    #[test]
    fn timestamps_track_newest_contributing_point() {
        // 40 points -> 3 AC values at ending indices 37,38,39 -> ts 137,138,139.
        let points: Vec<ResampledPointD1> = (0..40)
            .map(|i| rp_median(i + 1, (i + 100) as i64, i as f64))
            .collect();
        let result = compute(&points).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].timestamp, 137);
        assert_eq!(vals[1].timestamp, 138);
        assert_eq!(vals[2].timestamp, 139);
    }

    #[test]
    fn output_length_is_points_minus_37() {
        // 50 points -> 50 - 37 = 13 AC values.
        let points: Vec<ResampledPointD1> = (0..50)
            .map(|i| rp_median(i + 1, (i + 1) as i64, i as f64))
            .collect();
        let result = compute(&points).unwrap();
        assert_eq!(indicator_values!(result).len(), 13);
    }

    // --- Level 2: real AC output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from real EURUSD data (frozen fixture in `test_support`).
    ///
    /// These values are **not ground-truth** — they were produced by running
    /// `ac::compute` on the 2400 real EURUSD `resampled_point_d1` rows and
    /// capturing the output. They serve as a regression baseline. The user
    /// will later compare them against an external source of truth.
    #[test]
    fn level2_real_ac_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points).unwrap();
        let vals = indicator_values!(result);

        // 2400 points -> 2363 AC values (2400 - 37).
        assert_eq!(vals.len(), 2363);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1359072000);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1359072000, -0.0006758000000001377),
            (1359331200, 0.0005994529411764479),
            (1359417600, 0.0015999176470589578),
            (1359504000, 0.0036922588235297933),
            (1359590400, 0.005059188235293943),
            (1359676800, 0.005819888235294179),
            (1359936000, 0.004748764705882501),
            (1360022400, 0.003015517647058718),
            (1360108800, 0.0009001588235294719),
            (1360195200, -0.0019731647058822688),
            (1360281600, -0.006230041176470413),
            (1360540800, -0.007949582352941097),
            (1360627200, -0.007525770588235538),
            (1360713600, -0.006176770588235358),
            (1360800000, -0.0050496058823526375),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
