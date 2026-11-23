//! Relative Vigor Index: the RVI line crossing its signal line.
//!
//! Each bar's body `close - open` and range `high - low` are smoothed over
//! the last four bars with symmetric 1-2-2-1 weights.
//! `RVI = sum(body) / sum(range)` with both sums over the last `period`
//! smoothed bars; the signal line is the same 1-2-2-1 weighting of the RVI
//! line, divided by the weight total. Ported from `RVI.mq5`.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{CrossoverBuffers, IndicatorValue};
use crate::validate_period;

const SYMMETRIC_WEIGHTS: [f64; 4] = [1.0, 2.0, 2.0, 1.0];
const SYMMETRIC_WEIGHT_TOTAL: f64 = 6.0;

#[derive(Debug, Clone, Copy)]
struct SmoothedBar {
    timestamp: i64,
    body: f64,
    range: f64,
}

/// The long buffer is the RVI line and the short buffer its signal line.
///
/// The first RVI values, whose signal does not exist yet, are dropped so both
/// buffers share timestamps. Short input yields empty buffers, never an error.
pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<CrossoverBuffers, IndicatorError> {
    validate_period!(period, InvalidRviPeriod);

    let smoothed_bars: Vec<SmoothedBar> = points
        .windows(SYMMETRIC_WEIGHTS.len())
        .map(smoothed_bar)
        .collect();

    let rvi: Vec<IndicatorValue> = smoothed_bars
        .windows(cast::i32_to_usize(period))
        .map(rvi_point)
        .collect();

    let signal: Vec<IndicatorValue> = rvi
        .windows(SYMMETRIC_WEIGHTS.len())
        .map(signal_point)
        .collect();

    Ok(CrossoverBuffers {
        upcross_long_buffer: rvi
            .into_iter()
            .skip(SYMMETRIC_WEIGHTS.len().saturating_sub(1))
            .collect(),
        upcross_short_buffer: signal,
    })
}

fn smoothed_bar(window: &[ResampledPointD1]) -> SmoothedBar {
    let mut body: f64 = 0.0;
    let mut range: f64 = 0.0;
    let mut timestamp: i64 = 0;
    for (point, weight) in window.iter().zip(SYMMETRIC_WEIGHTS) {
        body = weight.mul_add(point.close - point.open, body);
        range = weight.mul_add(point.high - point.low, range);
        timestamp = point.timestamp;
    }

    SmoothedBar {
        timestamp,
        body,
        range,
    }
}

fn rvi_point(window: &[SmoothedBar]) -> IndicatorValue {
    let mut body: f64 = 0.0;
    let mut range: f64 = 0.0;
    let mut timestamp: i64 = 0;
    for bar in window {
        body += bar.body;
        range += bar.range;
        timestamp = bar.timestamp;
    }

    // MQL5 falls back to the bare body sum when the window has no range; on
    // bars whose open and close lie within high and low, that sum is 0.
    let value = if range == 0.0 { body } else { body / range };

    IndicatorValue { timestamp, value }
}

fn signal_point(window: &[IndicatorValue]) -> IndicatorValue {
    let mut weighted_rvi: f64 = 0.0;
    let mut timestamp: i64 = 0;
    for (rvi, weight) in window.iter().zip(SYMMETRIC_WEIGHTS) {
        weighted_rvi = weight.mul_add(rvi.value, weighted_rvi);
        timestamp = rvi.timestamp;
    }

    IndicatorValue {
        timestamp,
        value: weighted_rvi / SYMMETRIC_WEIGHT_TOTAL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;

    fn rp(id: i32, ts: i64, open: f64, high: f64, low: f64, close: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open,
            high,
            low,
            close,
            volume: 100,
            timestamp: ts,
        }
    }

    /// Every bar opens at its low and closes at its high: all vigor, upward.
    fn bullish_points(count: i32) -> Vec<ResampledPointD1> {
        (1..=count)
            .map(|i| rp(i, i64::from(i), 1.0, 2.0, 1.0, 2.0))
            .collect()
    }

    fn values(buffer: &[IndicatorValue]) -> Vec<f64> {
        buffer.iter().map(|v| v.value).collect()
    }

    fn timestamps(buffer: &[IndicatorValue]) -> Vec<i64> {
        buffer.iter().map(|v| v.timestamp).collect()
    }

    fn bar(body: f64, range: f64) -> SmoothedBar {
        SmoothedBar {
            timestamp: 0,
            body,
            range,
        }
    }

    fn iv(timestamp: i64, value: f64) -> IndicatorValue {
        IndicatorValue { timestamp, value }
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn close_at_high_from_low_open_yields_max_rvi() {
        let buffers = compute(&bullish_points(9), 2).unwrap();
        assert_eq!(values(&buffers.upcross_long_buffer), vec![1.0, 1.0]);
        assert_eq!(values(&buffers.upcross_short_buffer), vec![1.0, 1.0]);
    }

    #[test]
    fn close_at_low_from_high_open_yields_min_rvi() {
        let points: Vec<ResampledPointD1> = (1..=8)
            .map(|i| rp(i, i64::from(i), 2.0, 2.0, 1.0, 1.0))
            .collect();
        let buffers = compute(&points, 2).unwrap();
        assert_eq!(values(&buffers.upcross_long_buffer), vec![-1.0]);
        assert_eq!(values(&buffers.upcross_short_buffer), vec![-1.0]);
    }

    #[test]
    fn flat_market_yields_zero_rvi_like_mql5() {
        let points: Vec<ResampledPointD1> = (1..=8)
            .map(|i| rp(i, i64::from(i), 1.0, 1.0, 1.0, 1.0))
            .collect();
        let buffers = compute(&points, 2).unwrap();
        assert_eq!(values(&buffers.upcross_long_buffer), vec![0.0]);
        assert_eq!(values(&buffers.upcross_short_buffer), vec![0.0]);
    }

    #[test]
    fn inner_bars_weigh_double_in_smoothed_bar() {
        // Every range is 2; only the second bar has a body of 1.
        // Body 2 * 1 = 2, range 2 + 4 + 4 + 2 = 12.
        let points = vec![
            rp(1, 1, 1.0, 2.0, 0.0, 1.0),
            rp(2, 2, 1.0, 2.0, 0.0, 2.0),
            rp(3, 3, 1.0, 2.0, 0.0, 1.0),
            rp(4, 4, 1.0, 2.0, 0.0, 1.0),
        ];
        let smoothed = smoothed_bar(&points);
        assert_eq!(smoothed.body, 2.0);
        assert_eq!(smoothed.range, 12.0);
        assert_eq!(smoothed.timestamp, 4);
    }

    #[test]
    fn rvi_sums_bodies_and_ranges_before_dividing() {
        // (1 + 0) / (2 + 6) = 0.125; averaging per-bar ratios would give 0.25.
        let point = rvi_point(&[bar(1.0, 2.0), bar(0.0, 6.0)]);
        assert_eq!(point.value, 0.125);
    }

    #[test]
    fn signal_weighs_inner_rvi_values_double() {
        let point = signal_point(&[iv(1, 0.0), iv(2, 3.0), iv(3, 0.0), iv(4, 0.0)]);
        assert_eq!(point, iv(4, 1.0));
    }

    #[test]
    fn buffers_start_where_signal_line_starts() {
        // MQL5 starts the signal at bar period + 5 (0-based): 3 bars smooth
        // the vigor, period - 1 more sum it, 3 more smooth the signal.
        let buffers = compute(&bullish_points(10), 2).unwrap();
        assert_eq!(timestamps(&buffers.upcross_long_buffer), vec![8, 9, 10]);
        assert_eq!(timestamps(&buffers.upcross_short_buffer), vec![8, 9, 10]);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yield_empty_buffers() {
        let buffers = compute(&[], 10).unwrap();
        assert!(buffers.upcross_long_buffer.is_empty());
        assert!(buffers.upcross_short_buffer.is_empty());
    }

    #[test]
    fn one_point_short_of_warm_up_yields_empty_buffers() {
        // Period 2 needs 2 + 6 = 8 points.
        let buffers = compute(&bullish_points(7), 2).unwrap();
        assert!(buffers.upcross_long_buffer.is_empty());
        assert!(buffers.upcross_short_buffer.is_empty());
    }

    #[test]
    fn exactly_warm_up_points_yield_single_value_at_last_bar() {
        let buffers = compute(&bullish_points(8), 2).unwrap();
        assert_eq!(timestamps(&buffers.upcross_long_buffer), vec![8]);
        assert_eq!(timestamps(&buffers.upcross_short_buffer), vec![8]);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = bullish_points(2);
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidRviPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidRviPeriod { n: -3 }
        );
    }

    // --- Level 2: real RVI output on real EURUSD data (captured baseline) ---

    /// Baseline captured on the frozen EURUSD fixture with `period = 10` (the
    /// MQL5 default). A direct Python transcription of the `RVI.mq5` loops
    /// agrees to within 2e-16: the loops sum in a different order. Not checked
    /// against MetaTrader output itself.
    #[test]
    fn level2_real_rvi_pair_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let buffers = compute(&points, 10).unwrap();
        let long = &buffers.upcross_long_buffer;
        let short = &buffers.upcross_short_buffer;

        // 2400 points -> 2400 - 3 - (10 - 1) - 3 aligned values.
        assert_eq!(long.len(), 2385);
        assert_eq!(short.len(), 2385);

        let last_long = long.last().expect("non-empty");
        let last_short = short.last().expect("non-empty");
        assert_eq!(last_long.timestamp, 1646352000);
        assert_eq!(last_short.timestamp, 1646352000);
        assert_eq!(last_long.value, -0.12433738822202924);
        assert_eq!(last_short.value, -0.05816951666865853);

        let expected_first_5: &[(i64, f64, f64)] = &[
            (1356480000, 0.29546040497392445, 0.2585818625378626),
            (1356566400, 0.2563609408182114, 0.2790324423565443),
            (1356652800, 0.21346433182698982, 0.26804120928799463),
            (1356912000, 0.15696552284594503, 0.23201274551837867),
            (1356998400, 0.10032129175763653, 0.18292365698695293),
        ];
        for (i, (ts, l, s)) in expected_first_5.iter().enumerate() {
            assert_eq!(long[i].timestamp, *ts, "long ts mismatch at {i}");
            assert_eq!(short[i].timestamp, *ts, "short ts mismatch at {i}");
            assert_eq!(long[i].value, *l, "long value mismatch at {i}");
            assert_eq!(short[i].value, *s, "short value mismatch at {i}");
        }
    }
}
