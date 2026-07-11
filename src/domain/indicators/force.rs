use crate::domain::indicators::moving_averages::sma;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OneBuffer};

/// Compute the Force Index from resampled daily points.
///
/// `F[i] = volume[i] * (MA[i] - MA[i - 1])` where `MA = SMA(close, period)`
/// computed via the shared `sma` helper from `moving_averages.rs` (MQL5
/// `Force_Index.mq5`, `MODE_SMA` on `PRICE_CLOSE`, `VOLUME_TICK`).
///
/// SMA entry `j` corresponds to original index `period - 1 + j`. Force Index
/// requires both `MA[j]` (current) and `MA[j - 1]` (previous SMA entry), so
/// `F[j] = volume[period - 1 + j] * (MA[j] - MA[j - 1])` for
/// `j = 1..(MA.len())`. The MQL5 source sets `start = InpForcePeriod` (= the
/// index of the first Force Index value, 0-indexed), which is `period` (since
/// SMA entry `j = 1` is at original index `period - 1 + 1 = period`).
///
/// The MQL5 source sets `PLOT_DRAW_BEGIN = InpForcePeriod` (= `period`); warm-up
/// bars produce no `IndicatorValue` entries (per umbrella PRD Decision 10).
/// Each value is tagged with the timestamp of the resampled point at the
/// evaluated index.
///
/// Insufficient data (fewer than `period + 1` points, i.e. `ma` has fewer than
/// 2 entries) yields an empty `items` vector (not an error), per umbrella PRD
/// Decision 10. A non-positive `period` is an error.
pub fn compute(points: &[ResampledPointD1], period: i32) -> Result<OneBuffer, IndicatorError> {
    if period <= 0 {
        return Err(IndicatorError::InvalidForcePeriod { n: period });
    }
    let p = period as usize;

    let closes: Vec<f64> = points.iter().map(|pt| pt.close).collect();
    let ma = sma(&closes, p);

    if ma.len() < 2 {
        return Ok(OneBuffer { items: Vec::new() });
    }

    let items = (1..ma.len())
        .map(|j| {
            let i = p - 1 + j;
            IndicatorValue {
                timestamp: points[i].timestamp,
                value: points[i].volume as f64 * (ma[j] - ma[j - 1]),
            }
        })
        .collect();

    Ok(OneBuffer { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};

    /// Build a resampled point with an explicit close and volume (the two
    /// components the Force Index computation consumes).
    fn rp(id: i32, ts: i64, close: f64, volume: i32) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: close,
            high: close,
            low: close,
            close,
            volume,
            timestamp: ts,
            high_low_order: HighLowOrder::HighFirst,
        }
    }

    fn values(buffer: &OneBuffer) -> &[IndicatorValue] {
        &buffer.items
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_close_constant_volume_yields_zero_force() {
        // All bars: close=10, volume=100, period=3.
        // SMA([10,10,10,10,10], 3) = [10.0, 10.0, 10.0] (length 3).
        // F[j] = volume[2+j] * (MA[j] - MA[j-1]) = 100 * 0.0 = 0.0 for j=1,2.
        // 5 points -> 2 Force Index values = 0.0 at points[3].ts, points[4].ts.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, 10.0, 100))
            .collect();
        let result = compute(&points, 3).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[1].value, 0.0);
        assert_eq!(vals[1].timestamp, 5);
    }

    #[test]
    fn linearly_increasing_close_constant_volume_yields_hand_computed_force() {
        // close[i] = i + 1, volume=100, period=3.
        // SMA([1,2,3,4,5], 3) = [2.0, 3.0, 4.0] (length 3).
        // F[1] = volume[3] * (MA[1] - MA[0]) = 100 * (3.0 - 2.0) = 100.0
        //   at points[3].ts.
        // F[2] = volume[4] * (MA[2] - MA[1]) = 100 * (4.0 - 3.0) = 100.0
        //   at points[4].ts.
        // 5 points -> 2 Force Index values = 100.0 each.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, (i + 1) as f64, 100))
            .collect();
        let result = compute(&points, 3).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].value, 100.0);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[1].value, 100.0);
        assert_eq!(vals[1].timestamp, 5);
    }

    #[test]
    fn alternating_close_constant_volume_yields_zero_force() {
        // close = [1, 3, 1, 3, 1], volume=50, period=2.
        // SMA([1,3,1,3,1], 2) = [2.0, 2.0, 2.0, 2.0] (length 4).
        // F[j] = volume[1+j] * (MA[j] - MA[j-1]) = 50 * 0.0 = 0.0 for j=1,2,3.
        // 5 points -> 3 Force Index values = 0.0 each.
        let closes = [1.0, 3.0, 1.0, 3.0, 1.0];
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, closes[i as usize], 50))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 3);
        assert_eq!(vals[1].value, 0.0);
        assert_eq!(vals[1].timestamp, 4);
        assert_eq!(vals[2].value, 0.0);
        assert_eq!(vals[2].timestamp, 5);
    }

    #[test]
    fn non_constant_ma_delta_yields_hand_computed_force() {
        // close = [1, 2, 4, 8, 16], volume=10, period=2.
        // SMA([1,2,4,8,16], 2) = [1.5, 3.0, 6.0, 12.0] (length 4).
        // F[1] = 10 * (3.0 - 1.5) = 15.0  at points[2].ts.
        // F[2] = 10 * (6.0 - 3.0) = 30.0  at points[3].ts.
        // F[3] = 10 * (12.0 - 6.0) = 60.0 at points[4].ts.
        // 5 points -> 3 Force Index values = 15.0, 30.0, 60.0.
        let closes = [1.0, 2.0, 4.0, 8.0, 16.0];
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, closes[i as usize], 10))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].value, 15.0);
        assert_eq!(vals[0].timestamp, 3);
        assert_eq!(vals[1].value, 30.0);
        assert_eq!(vals[1].timestamp, 4);
        assert_eq!(vals[2].value, 60.0);
        assert_eq!(vals[2].timestamp, 5);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[], 13).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_period_plus_one_points_yields_empty_buffer() {
        // period=13, 13 points -> SMA has 1 entry -> no Force Index values.
        let points: Vec<ResampledPointD1> = (0..13)
            .map(|i| rp(i + 1, (i + 1) as i64, 10.0, 100))
            .collect();
        let result = compute(&points, 13).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_points_yields_empty_buffer() {
        // period=13, 13 points -> SMA has 1 entry -> no Force Index values.
        let points: Vec<ResampledPointD1> = (0..13)
            .map(|i| rp(i + 1, (i + 1) as i64, 10.0, 100))
            .collect();
        let result = compute(&points, 13).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_plus_one_points_yields_single_value() {
        // period=3, 4 points -> SMA has 2 entries -> 1 Force Index value.
        // SMA([10,10,10,10], 3) = [10.0, 10.0]. F[1] = 100 * 0.0 = 0.0.
        let points: Vec<ResampledPointD1> = (0..4)
            .map(|i| rp(i + 1, (i + 1) as i64, 10.0, 100))
            .collect();
        let result = compute(&points, 3).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 4);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = vec![rp(1, 1, 10.0, 100)];
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidForcePeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidForcePeriod { n: -3 }
        );
    }

    #[test]
    fn timestamps_track_evaluated_bar() {
        // period=3, 5 points -> SMA has 3 entries -> 2 Force Index values at
        // original indices 3 and 4 -> timestamps points[3].ts, points[4].ts.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 100) as i64, 10.0, 100))
            .collect();
        let result = compute(&points, 3).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].timestamp, 103);
        assert_eq!(vals[1].timestamp, 104);
    }

    #[test]
    fn output_length_is_points_minus_period() {
        // 10 points, period 3 -> 10 - 3 = 7 Force Index values.
        let points: Vec<ResampledPointD1> = (0..10)
            .map(|i| rp(i + 1, (i + 1) as i64, 10.0, 100))
            .collect();
        let result = compute(&points, 3).unwrap();
        assert_eq!(values(&result).len(), 7);
    }

    // --- Level 2: real Force Index output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from the real EURUSD development DB.
    ///
    /// These values are **not ground-truth** — they were produced by running
    /// `force::compute` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `period = 13` (the MQL5 default) and capturing the output. They serve
    /// as a regression baseline. The user will later compare them against an
    /// external source of truth.
    #[test]
    fn level2_real_force_output_matches_captured_baseline() {
        let points = match test_support::fetch_eurusd_points() {
            Some(p) => p,
            None => {
                eprintln!("skipping level2_baseline_test: development DB unavailable");
                return;
            }
        };
        let result = compute(&points, 13).unwrap();
        let vals = values(&result);

        // 2400 points -> 2387 Force Index values (2400 - 13).
        assert_eq!(vals.len(), 2387);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1356048000);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1356048000, 44.11657846155105),
            (1356307200, 29.043355384609814),
            (1356480000, 56.355169230772816),
            (1356566400, 114.33590999999919),
            (1356652800, 95.4192753846078),
            (1356912000, 53.03432000000902),
            (1356998400, 1.435257692307593),
            (1357084800, 73.35510230768551),
            (1357171200, -48.80912615384578),
            (1357257600, -34.7051261538531),
            (1357516800, -36.91164999998668),
            (1357603200, -45.616787692320806),
            (1357689600, -71.78677846154238),
            (1357776000, 33.219934615404256),
            (1357862400, 65.18054923075438),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
