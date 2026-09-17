use crate::domain::indicators::moving_averages::sma;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorSeries};

/// Fast SMA period (fixed per the MQL5 `Awesome_Oscillator.mq5` source).
const FAST_PERIOD: usize = 5;
/// Slow SMA period (fixed per the MQL5 `Awesome_Oscillator.mq5` source).
const SLOW_PERIOD: usize = 34;

/// Compute the Awesome Oscillator from resampled daily points.
///
/// `AO[i] = SMA(median, 5)[i] − SMA(median, 34)[i]` where
/// `median = (high + low) / 2` (MQL5 `PRICE_MEDIAN`). The first valid AO
/// value is at index 33 (the 34th bar); warm-up bars produce no
/// `IndicatorValue` entries. Each value is tagged with the timestamp of the
/// resampled point at the index of the newest contributing bar.
///
/// Insufficient data (fewer than 34 points) yields an empty buffer, not an
/// error — consistent with the RSI reference and umbrella PRD Decision 10.
pub fn compute(points: &[ResampledPointD1]) -> Result<OscillatorSeries, IndicatorError> {
    let medians: Vec<f64> = points
        .iter()
        .map(|p| f64::midpoint(p.high, p.low))
        .collect();

    let fast = sma(&medians, FAST_PERIOD);
    let slow = sma(&medians, SLOW_PERIOD);

    // fast[k] ends at index k + FAST_PERIOD - 1 (= k + 4).
    // slow[k] ends at index k + SLOW_PERIOD - 1 (= k + 33).
    // AO at ending index i pairs slow[k] with fast[k + offset] where
    // offset = SLOW_PERIOD - FAST_PERIOD (= 29). AO is defined for ending
    // indices 33..n, i.e. one value per slow entry.
    let offset = SLOW_PERIOD - FAST_PERIOD;

    // AO at ending index k + SLOW_PERIOD - 1 pairs slow[k] with
    // fast[k + offset].
    let items = slow
        .iter()
        .zip(fast.iter().skip(offset))
        .zip(points.iter().skip(SLOW_PERIOD - 1))
        .map(|((slow_val, fast_val), point)| IndicatorValue {
            timestamp: point.timestamp,
            value: fast_val - slow_val,
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
    fn constant_median_yields_zero_ao() {
        // All medians equal 5.0 -> SMA(5)=5, SMA(34)=5 -> AO=0 everywhere.
        let points: Vec<ResampledPointD1> = (0..34)
            .map(|i| rp_median(i + 1, i64::from(i + 1), 5.0))
            .collect();
        let result = compute(&points).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 34);
    }

    #[test]
    fn linear_ramp_median_yields_constant_ao() {
        // median[i] = i (via high=i+0.5, low=i-0.5).
        // SMA(median,5)[ending at i] = i - 2
        // SMA(median,34)[ending at i] = i - 16.5
        // AO[i] = (i-2) - (i-16.5) = 14.5 for all i >= 33.
        let points: Vec<ResampledPointD1> = (0..35)
            .map(|i| rp_median(i + 1, i64::from(i + 100), f64::from(i)))
            .collect();
        let result = compute(&points).unwrap();
        let vals = indicator_values!(result);
        // 35 points -> AO at ending indices 33,34 -> 2 values.
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].value, 14.5);
        assert_eq!(vals[1].value, 14.5);
        // timestamps are points[33].ts and points[34].ts
        assert_eq!(vals[0].timestamp, 133);
        assert_eq!(vals[1].timestamp, 134);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[]).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_34_points_yields_empty_buffer() {
        // 33 points -> slow SMA has no full window -> empty.
        let points: Vec<ResampledPointD1> = (0..33)
            .map(|i| rp_median(i + 1, i64::from(i + 1), 1.0))
            .collect();
        let result = compute(&points).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_34_points_yields_single_value() {
        let points: Vec<ResampledPointD1> = (0..34)
            .map(|i| rp_median(i + 1, i64::from(i + 1), 1.0))
            .collect();
        let result = compute(&points).unwrap();
        assert_eq!(indicator_values!(result).len(), 1);
        // constant median -> AO = 0
        assert_eq!(indicator_values!(result)[0].value, 0.0);
        // timestamp of the 34th point (index 33)
        assert_eq!(indicator_values!(result)[0].timestamp, 34);
    }

    #[test]
    fn timestamps_track_newest_contributing_point() {
        // Distinct timestamps; verify AO values carry the right ones.
        // 36 points -> AO at ending indices 33,34,35 -> timestamps 133,134,135.
        let points: Vec<ResampledPointD1> = (0..36)
            .map(|i| rp_median(i + 1, i64::from(i + 100), f64::from(i)))
            .collect();
        let result = compute(&points).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].timestamp, 133);
        assert_eq!(vals[1].timestamp, 134);
        assert_eq!(vals[2].timestamp, 135);
    }

    #[test]
    fn output_length_is_points_minus_33() {
        // 40 points -> 40 - 33 = 7 AO values.
        let points: Vec<ResampledPointD1> = (0..40)
            .map(|i| rp_median(i + 1, i64::from(i + 1), f64::from(i)))
            .collect();
        let result = compute(&points).unwrap();
        assert_eq!(indicator_values!(result).len(), 7);
    }

    // --- Level 2: real AO output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from real EURUSD data (frozen fixture in `test_support`).
    ///
    /// These values are **not ground-truth** — they were produced by running
    /// `ao::compute` on the 2400 real EURUSD `resampled_point_d1` rows and
    /// capturing the output. They serve as a regression baseline. The user
    /// will later compare them against an external source of truth.
    #[test]
    fn level2_real_ao_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points).unwrap();
        let vals = indicator_values!(result);

        // 2400 points -> 2367 AO values (2400 - 33).
        assert_eq!(vals.len(), 2367);

        // First and last timestamps.
        assert_eq!(vals[0].timestamp, 1358726400);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1358726400, 0.015506323529411725),
            (1358812800, 0.014576794117646674),
            (1358899200, 0.014313882352940954),
            (1358985600, 0.013583176470587821),
            (1359072000, 0.013650294117646622),
            (1359331200, 0.014780352941176078),
            (1359417600, 0.016081823529411565),
            (1359504000, 0.019139235294117762),
            (1359590400, 0.022236911764705436),
            (1359676800, 0.025334441176470435),
            (1359936000, 0.026634058823529427),
            (1360022400, 0.027105558823529163),
            (1360108800, 0.026452941176470457),
            (1360195200, 0.023915294117647035),
            (1360281600, 0.018239411764706004),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
