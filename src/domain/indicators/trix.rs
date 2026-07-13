use crate::domain::indicators::moving_averages::ema;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OneBuffer};

/// Compute the TRIX (Triple Exponential Average) from resampled daily points.
///
/// `EMA3 = EMA(EMA(EMA(close, period)))` and
/// `TRIX[k] = (EMA3[k + 1] - EMA3[k]) / EMA3[k]` when `EMA3[k] != 0.0`,
/// otherwise `0.0` (MQL5 `TRIX.mq5`, `PRICE_CLOSE` baked in via
/// `#property indicator_applied_price PRICE_CLOSE`). The `ema` helper from
/// `moving_averages.rs` is reused three times — once for each EMA layer.
///
/// EMA3 entry `j` corresponds to original index `3 * (period - 1) + j`. TRIX
/// requires both `EMA3[k]` (previous) and `EMA3[k + 1]` (current) to be valid,
/// so the first TRIX value is at original index `3 * period - 2`. The MQL5
/// source sets `PLOT_DRAW_BEGIN = 3 * InpPeriodEMA - 3` (= `3 * period - 3`);
/// warm-up bars produce no `IndicatorValue` entries (per umbrella PRD Decision
/// 10). Each value is tagged with the timestamp of the resampled point at the
/// evaluated index.
///
/// Insufficient data (fewer than `3 * period - 1` points, i.e. `EMA3` has
/// fewer than 2 entries) yields an empty `items` vector (not an error), per
/// umbrella PRD Decision 10. A non-positive `period` is an error.
pub fn compute(points: &[ResampledPointD1], period: i32) -> Result<OneBuffer, IndicatorError> {
    if period <= 0 {
        return Err(IndicatorError::InvalidTrixPeriod { n: period });
    }
    let p = period as usize;

    let closes: Vec<f64> = points.iter().map(|pt| pt.close).collect();
    let ema1 = ema(&closes, p);
    let ema2 = ema(&ema1, p);
    let ema3 = ema(&ema2, p);

    if ema3.len() < 2 {
        return Ok(OneBuffer { items: Vec::new() });
    }

    // TRIX entry k corresponds to original index 3*p - 2 + k.
    let start = 3 * p - 2;
    let items = (0..(ema3.len() - 1))
        .map(|k| {
            let value = if ema3[k] == 0.0 {
                0.0
            } else {
                (ema3[k + 1] - ema3[k]) / ema3[k]
            };
            IndicatorValue {
                timestamp: points[start + k].timestamp,
                value,
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

    /// Build a resampled point with an explicit close (the only component the
    /// TRIX computation consumes).
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

    fn values(buffer: &OneBuffer) -> &[IndicatorValue] {
        &buffer.items
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_close_yields_zero_trix() {
        // All bars: close=10, period=2, N=8.
        // EMA1 = [10.0;7], EMA2 = [10.0;6], EMA3 = [10.0;5].
        // TRIX[k] = (10.0 - 10.0) / 10.0 = 0.0 for all k.
        // 8 points -> 4 TRIX values = 0.0 at points[4].ts, points[5].ts,
        // points[6].ts, points[7].ts.
        let points: Vec<ResampledPointD1> =
            (0..8).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 2).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 5);
        assert_eq!(vals[1].value, 0.0);
        assert_eq!(vals[1].timestamp, 6);
        assert_eq!(vals[2].value, 0.0);
        assert_eq!(vals[2].timestamp, 7);
        assert_eq!(vals[3].value, 0.0);
        assert_eq!(vals[3].timestamp, 8);
    }

    #[test]
    fn linearly_increasing_close_yields_hand_computed_trix() {
        // close[i] = i + 1, period=2, N=8.
        // EMA1 = [1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5] (len 7).
        // EMA2 = [2.0, 3.0, 4.0, 5.0, 6.0, 7.0] (len 6).
        // EMA3 = [2.5, 3.5, 4.5, 5.5, 6.5] (len 5).
        // TRIX[0] = (3.5 - 2.5) / 2.5 = 0.4 at points[4].ts.
        // TRIX[1] = (4.5 - 3.5) / 3.5 = 0.285714... at points[5].ts.
        // TRIX[2] = (5.5 - 4.5) / 4.5 = 0.222222... at points[6].ts.
        // TRIX[3] = (6.5 - 5.5) / 5.5 = 0.181818... at points[7].ts.
        // 8 points -> 4 TRIX values.
        let points: Vec<ResampledPointD1> = (0..8)
            .map(|i| rp(i + 1, (i + 1) as i64, (i + 1) as f64))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 0.4);
        assert_eq!(vals[0].timestamp, 5);
        assert!((vals[1].value - 0.2857142857142857).abs() < 1e-9);
        assert_eq!(vals[1].timestamp, 6);
        assert!((vals[2].value - 0.2222222222222222).abs() < 1e-9);
        assert_eq!(vals[2].timestamp, 7);
        assert!((vals[3].value - 0.18181818181818182).abs() < 1e-9);
        assert_eq!(vals[3].timestamp, 8);
    }

    #[test]
    fn decreasing_close_yields_negative_trix() {
        // close = [8, 7, 6, 5, 4, 3, 2, 1], period=2, N=8.
        // EMA1 = [7.5, 6.5, 5.5, 4.5, 3.5, 2.5, 1.5] (len 7).
        // EMA2 = [7.0, 6.0, 5.0, 4.0, 3.0, 2.0] (len 6).
        // EMA3 = [6.5, 5.5, 4.5, 3.5, 2.5] (len 5).
        // TRIX[0] = (5.5 - 6.5) / 6.5 = -0.153846... at points[4].ts.
        // TRIX[1] = (4.5 - 5.5) / 5.5 = -0.181818... at points[5].ts.
        // TRIX[2] = (3.5 - 4.5) / 4.5 = -0.222222... at points[6].ts.
        // TRIX[3] = (2.5 - 3.5) / 3.5 = -0.285714... at points[7].ts.
        // 8 points -> 4 TRIX values.
        let closes = [8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0];
        let points: Vec<ResampledPointD1> = (0..8)
            .map(|i| rp(i + 1, (i + 1) as i64, closes[i as usize]))
            .collect();
        let result = compute(&points, 2).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 4);
        assert!((vals[0].value - (-0.15384615384615385)).abs() < 1e-9);
        assert_eq!(vals[0].timestamp, 5);
        assert!((vals[1].value - (-0.18181818181818182)).abs() < 1e-9);
        assert_eq!(vals[1].timestamp, 6);
        assert!((vals[2].value - (-0.2222222222222222)).abs() < 1e-9);
        assert_eq!(vals[2].timestamp, 7);
        assert!((vals[3].value - (-0.2857142857142857)).abs() < 1e-9);
        assert_eq!(vals[3].timestamp, 8);
    }

    #[test]
    fn zero_price_zero_division_guard_yields_zero_trix() {
        // All bars: close=0, period=2, N=8.
        // EMA1 = [0.0;7], EMA2 = [0.0;6], EMA3 = [0.0;5].
        // TRIX[k]: EMA3[k] = 0.0 -> zero-division guard -> 0.0 for all k.
        // 8 points -> 4 TRIX values = 0.0.
        let points: Vec<ResampledPointD1> =
            (0..8).map(|i| rp(i + 1, (i + 1) as i64, 0.0)).collect();
        let result = compute(&points, 2).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 5);
        assert_eq!(vals[1].value, 0.0);
        assert_eq!(vals[1].timestamp, 6);
        assert_eq!(vals[2].value, 0.0);
        assert_eq!(vals[2].timestamp, 7);
        assert_eq!(vals[3].value, 0.0);
        assert_eq!(vals[3].timestamp, 8);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[], 14).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_three_period_minus_one_points_yields_empty_buffer() {
        // period=2, 3*2 - 1 = 5 points needed for a single value.
        // 3 points -> EMA3 has 0 entries -> no TRIX values.
        let points: Vec<ResampledPointD1> =
            (0..3).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 2).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_three_period_minus_two_points_yields_empty_buffer() {
        // period=2, 3*2 - 2 = 4 points -> EMA3 has 1 entry -> no TRIX values.
        let points: Vec<ResampledPointD1> =
            (0..4).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 2).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_three_period_minus_one_points_yields_single_value() {
        // period=2, 3*2 - 1 = 5 points -> EMA3 has 2 entries -> 1 TRIX value.
        // Constant close=10 -> all EMAs = 10.0 -> TRIX = 0.0.
        let points: Vec<ResampledPointD1> =
            (0..5).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 2).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 5);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = vec![rp(1, 1, 10.0)];
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidTrixPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidTrixPeriod { n: -3 }
        );
    }

    #[test]
    fn timestamps_track_evaluated_bar() {
        // period=2, 8 points -> 4 TRIX values at original indices 4, 5, 6, 7
        // -> timestamps points[4].ts, points[5].ts, points[6].ts,
        // points[7].ts.
        let points: Vec<ResampledPointD1> =
            (0..8).map(|i| rp(i + 1, (i + 100) as i64, 10.0)).collect();
        let result = compute(&points, 2).unwrap();
        let vals = values(&result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].timestamp, 104);
        assert_eq!(vals[1].timestamp, 105);
        assert_eq!(vals[2].timestamp, 106);
        assert_eq!(vals[3].timestamp, 107);
    }

    #[test]
    fn output_length_is_n_minus_three_period_plus_two() {
        // 10 points, period 2 -> 10 - 3*2 + 2 = 6 TRIX values.
        let points: Vec<ResampledPointD1> =
            (0..10).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 2).unwrap();
        assert_eq!(values(&result).len(), 6);
    }

    // --- Level 2: real TRIX output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from the real EURUSD development DB.
    ///
    /// These values are **not ground-truth** — they were produced by running
    /// `trix::compute` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `period = 14` (the MQL5 default) and capturing the output. They serve
    /// as a regression baseline. The user will later compare them against an
    /// external source of truth.
    #[test]
    fn level2_real_trix_output_matches_captured_baseline() {
        let points = match test_support::fetch_eurusd_points() {
            Some(p) => p,
            None => {
                eprintln!("skipping level2_baseline_test: development DB unavailable");
                return;
            }
        };
        let result = compute(&points, 14).unwrap();
        let vals = values(&result);

        // 2400 points -> 2360 TRIX values (2400 - 3*14 + 2).
        assert_eq!(vals.len(), 2360);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1359504000);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1359504000, 0.0008663544705379816),
            (1359590400, 0.0009256118845248478),
            (1359676800, 0.0009951268447921824),
            (1359936000, 0.0010409799108784495),
            (1360022400, 0.0010808400862114538),
            (1360108800, 0.0011007575624136974),
            (1360195200, 0.0010820874115167463),
            (1360281600, 0.0010308428750488959),
            (1360540800, 0.0009656919922253115),
            (1360627200, 0.0009013493227282165),
            (1360713600, 0.0008379236678324667),
            (1360800000, 0.0007600023207534008),
            (1360886400, 0.0006748917332256184),
            (1361145600, 0.0005864279691686364),
            (1361232000, 0.0005046042394199655),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
