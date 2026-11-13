//! Detrended Price Oscillator (DPO): price with its trend removed.
//!
//! `close` minus the SMA of `period / 2 + 1` closes, keeping the source's
//! half-period shift so values line up with the bar they describe. Ported
//! from `DPO.mq5`.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::indicators::moving_averages::sma;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorBuffer};
use crate::empty_oscillator;
use crate::validate_period;

pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<OscillatorBuffer, IndicatorError> {
    validate_period!(period, InvalidDpoPeriod);
    let skip_count = cast::i32_to_usize(period / 2);
    let ext_ma_period = skip_count.saturating_add(1);

    let closes: Vec<f64> = points.iter().map(|p| p.close).collect();
    let sma_close = sma(&closes, ext_ma_period);

    if sma_close.is_empty() {
        empty_oscillator!();
    }

    let items = sma_close
        .iter()
        .zip(closes.iter().skip(skip_count))
        .zip(points.iter().skip(skip_count))
        .map(|((sma_val, close), point)| IndicatorValue {
            timestamp: point.timestamp,
            value: close - sma_val,
        })
        .collect();

    Ok(OscillatorBuffer { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::ResampledPointD1;
    use crate::indicator_values;

    /// Build a resampled point with an explicit close (the only price
    /// component the DPO computation consumes).
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
        }
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_close_yields_zero_dpo() {
        // All bars: close=10, period=4 -> ExtMAPeriod = 4/2 + 1 = 3.
        // SMA([10,10,10,10,10], 3) = [10.0, 10.0, 10.0] (length 3).
        // DPO[j] = close[2+j] - SMA[j] = 10 - 10 = 0.0 for all j.
        // 5 points -> 3 DPO values = 0.0 at points[2].ts, points[3].ts,
        // points[4].ts.
        let points: Vec<ResampledPointD1> =
            (0..5).map(|i| rp(i + 1, i64::from(i + 1), 10.0)).collect();
        let result = compute(&points, 4).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 3);
        assert_eq!(vals[1].value, 0.0);
        assert_eq!(vals[1].timestamp, 4);
        assert_eq!(vals[2].value, 0.0);
        assert_eq!(vals[2].timestamp, 5);
    }

    #[test]
    fn linearly_increasing_close_yields_hand_computed_dpo() {
        // close[i] = i + 1, period=4 -> ExtMAPeriod = 3.
        // SMA([1,2,3,4,5], 3) = [2.0, 3.0, 4.0] (length 3).
        // DPO[0] = close[2] - 2.0 = 3 - 2 = 1.0 at points[2].ts.
        // DPO[1] = close[3] - 3.0 = 4 - 3 = 1.0 at points[3].ts.
        // DPO[2] = close[4] - 4.0 = 5 - 4 = 1.0 at points[4].ts.
        // 5 points -> 3 DPO values = 1.0 each.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, i64::from(i + 1), f64::from(i + 1)))
            .collect();
        let result = compute(&points, 4).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].value, 1.0);
        assert_eq!(vals[0].timestamp, 3);
        assert_eq!(vals[1].value, 1.0);
        assert_eq!(vals[1].timestamp, 4);
        assert_eq!(vals[2].value, 1.0);
        assert_eq!(vals[2].timestamp, 5);
    }

    #[test]
    fn alternating_close_yields_hand_computed_dpo() {
        // close = [1, 3, 1, 3, 1], period=2 -> ExtMAPeriod = 2/2 + 1 = 2.
        // SMA([1,3,1,3,1], 2) = [2.0, 2.0, 2.0, 2.0] (length 4).
        // DPO[0] = close[1] - 2.0 = 3 - 2 = 1.0 at points[1].ts.
        // DPO[1] = close[2] - 2.0 = 1 - 2 = -1.0 at points[2].ts.
        // DPO[2] = close[3] - 2.0 = 3 - 2 = 1.0 at points[3].ts.
        // DPO[3] = close[4] - 2.0 = 1 - 2 = -1.0 at points[4].ts.
        // 5 points -> 4 DPO values alternating 1.0, -1.0.
        let closes = [1.0, 3.0, 1.0, 3.0, 1.0];
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, i64::from(i + 1), closes[usize::try_from(i).unwrap()]))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 1.0);
        assert_eq!(vals[0].timestamp, 2);
        assert_eq!(vals[1].value, -1.0);
        assert_eq!(vals[1].timestamp, 3);
        assert_eq!(vals[2].value, 1.0);
        assert_eq!(vals[2].timestamp, 4);
        assert_eq!(vals[3].value, -1.0);
        assert_eq!(vals[3].timestamp, 5);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[], 12).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_ext_ma_period_points_yields_empty_buffer() {
        // period=12 -> ExtMAPeriod = 12/2 + 1 = 7. 6 points -> SMA empty.
        let points: Vec<ResampledPointD1> =
            (0..6).map(|i| rp(i + 1, i64::from(i + 1), 10.0)).collect();
        let result = compute(&points, 12).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_over_two_points_yields_empty_buffer() {
        // period=12 -> period/2 = 6. 6 points -> ExtMAPeriod = 7 -> SMA empty.
        let points: Vec<ResampledPointD1> =
            (0..6).map(|i| rp(i + 1, i64::from(i + 1), 10.0)).collect();
        let result = compute(&points, 12).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_over_two_plus_one_points_yields_single_value() {
        // period=12 -> ExtMAPeriod = 7. 7 points -> SMA has 1 entry -> 1 DPO.
        let points: Vec<ResampledPointD1> =
            (0..7).map(|i| rp(i + 1, i64::from(i + 1), 10.0)).collect();
        let result = compute(&points, 12).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 7);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = vec![rp(1, 1, 10.0)];
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidDpoPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidDpoPeriod { n: -3 }
        );
    }

    #[test]
    fn timestamps_track_evaluated_bar() {
        // period=4 -> ExtMAPeriod = 3. 5 points -> SMA has 3 entries -> 3 DPO
        // values at original indices 2, 3, 4 -> timestamps points[2].ts,
        // points[3].ts, points[4].ts.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, i64::from(i + 100), 10.0))
            .collect();
        let result = compute(&points, 4).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].timestamp, 102);
        assert_eq!(vals[1].timestamp, 103);
        assert_eq!(vals[2].timestamp, 104);
    }

    #[test]
    fn output_length_is_points_minus_period_over_two() {
        // 10 points, period 4 -> 10 - 4/2 = 10 - 2 = 8 DPO values.
        let points: Vec<ResampledPointD1> =
            (0..10).map(|i| rp(i + 1, i64::from(i + 1), 10.0)).collect();
        let result = compute(&points, 4).unwrap();
        assert_eq!(indicator_values!(result).len(), 8);
    }

    // --- Level 2: real DPO output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from real EURUSD data (frozen fixture in `test_support`).
    ///
    /// These values are **not ground-truth**; they were produced by running
    /// `dpo::compute` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `period = 12` (the MQL5 default) and capturing the output. They serve
    /// as a regression baseline. The user will later compare them against an
    /// external source of truth.
    #[test]
    fn level2_real_dpo_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 12).unwrap();
        let vals = indicator_values!(result);

        // 2400 points -> 2394 DPO values (2400 - 12/2).
        assert_eq!(vals.len(), 2394);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1355270400);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1355270400, 0.006060000000000176),
            (1355356800, 0.006801428571428625),
            (1355443200, 0.01425571428571426),
            (1355702400, 0.011077142857143008),
            (1355788800, 0.013248571428571498),
            (1355875200, 0.007064285714285745),
            (1355961600, 0.008349999999999858),
            (1356048000, 0.0008099999999999774),
            (1356307200, -0.0007671428571429662),
            (1356480000, 0.0017328571428572026),
            (1356566400, 0.0020528571428570785),
            (1356652800, 0.000220000000000109),
            (1356912000, -0.0015685714285713637),
            (1356998400, -0.00010571428571415353),
            (1357084800, -0.0017857142857142794),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
