use crate::domain::indicators::moving_averages::ema;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorSeries};

/// Compute the Chaikin Volatility from resampled daily points.
///
/// `CHV[i] = 100 * (SHL[i] - SHL[i - chv_period]) / SHL[i - chv_period]` when
/// `SHL[i - chv_period] != 0.0`, otherwise `0.0`, where
/// `SHL = EMA(HL, smooth_period)` and `HL[i] = high[i] - low[i]`. The EMA is
/// computed via the shared `ema` helper from `moving_averages.rs` (MQL5
/// `CHV.mq5`, `InpSmoothType = EMA`).
///
/// SHL entry `j` corresponds to original index `smooth_period - 1 + j`. CHV
/// requires both `SHL[k + chv_period]` (current) and `SHL[k]`
/// (`chv_period` bars ago) to be valid, so the first CHV value is at original
/// index `smooth_period - 1 + chv_period`. The MQL5 source sets
/// `PLOT_DRAW_BEGIN = smooth_period + chv_period - 1`; warm-up bars produce
/// no `IndicatorValue` entries (per umbrella PRD Decision 10). Each value is
/// tagged with the timestamp of the resampled point at the evaluated index.
///
/// Insufficient data (SHL has `chv_period` or fewer entries, i.e. fewer than
/// `smooth_period + chv_period` points) yields an empty `items` vector (not
/// an error), per umbrella PRD Decision 10. A non-positive `smooth_period`
/// or `chv_period` is an error.
pub fn compute(
    points: &[ResampledPointD1],
    smooth_period: i32,
    chv_period: i32,
) -> Result<OscillatorSeries, IndicatorError> {
    if smooth_period <= 0 {
        return Err(IndicatorError::InvalidChvSmoothPeriod { n: smooth_period });
    }
    if chv_period <= 0 {
        return Err(IndicatorError::InvalidChvPeriod { n: chv_period });
    }
    let smooth = smooth_period as usize;
    let chv = chv_period as usize;

    let hl: Vec<f64> = points.iter().map(|p| p.high - p.low).collect();
    let shl = ema(&hl, smooth);

    // SHL entry j -> original index smooth - 1 + j.
    // CHV[k] uses SHL[k + chv] (current) and SHL[k] (chv_period bars ago).
    // Need at least chv + 1 SHL entries for one CHV value.
    if shl.len() < chv + 1 {
        return Ok(OscillatorSeries { items: Vec::new() });
    }

    let items = (0..(shl.len() - chv))
        .map(|k| {
            let prev = shl[k];
            let curr = shl[k + chv];
            let value = if prev != 0.0 {
                100.0 * (curr - prev) / prev
            } else {
                0.0
            };
            let i = smooth - 1 + k + chv;
            IndicatorValue {
                timestamp: points[i].timestamp,
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
    use crate::indicator_values;

    /// Build a resampled point with explicit high and low (the two
    /// components the CHV computation consumes).
    fn rp(id: i32, ts: i64, high: f64, low: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: low,
            high,
            low,
            close: high,
            volume: 1000,
            timestamp: ts,
            high_low_order: HighLowOrder::HighFirst,
        }
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_high_low_spread_yields_zero_chv() {
        // All bars: high=11, low=9, smooth_period=2, chv_period=3.
        // HL = [2, 2, 2, 2, 2]. SHL = EMA(HL, 2) = [2.0, 2.0, 2.0, 2.0].
        // CHV[0] = 100 * (SHL[3] - SHL[0]) / SHL[0] = 100 * 0 / 2 = 0.0.
        // 5 points -> 1 CHV value = 0.0 at points[4].ts.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, 11.0, 9.0))
            .collect();
        let result = compute(&points, 2, 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 5);
    }

    #[test]
    fn linearly_increasing_spread_yields_hand_computed_chv() {
        // All bars: high[i] = 10 + i, low[i] = 10, smooth_period=2, chv_period=2.
        // HL = [0, 1, 2, 3, 4]. SHL = EMA(HL, 2):
        //   seed = SMA(0, 1) = 0.5 (index 1);
        //   SHL[1] = (2 - 0.5) * (2/3) + 0.5 = 1.5;
        //   SHL[2] = (3 - 1.5) * (2/3) + 1.5 = 2.5;
        //   SHL[3] = (4 - 2.5) * (2/3) + 2.5 = 3.5.
        // CHV[0] = 100 * (2.5 - 0.5) / 0.5 = 400.0 at points[3].ts.
        // CHV[1] = 100 * (3.5 - 1.5) / 1.5 = 133.333... at points[4].ts.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, (10 + i) as f64, 10.0))
            .collect();
        let result = compute(&points, 2, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].value, 400.0);
        assert_eq!(vals[0].timestamp, 4);
        assert!((vals[1].value - 133.33333333333334).abs() < 1e-9);
        assert_eq!(vals[1].timestamp, 5);
    }

    #[test]
    fn zero_high_low_spread_yields_zero_chv_via_guard() {
        // All bars: high=10, low=10, smooth_period=2, chv_period=2.
        // HL = [0, 0, 0, 0, 0]. SHL = [0.0, 0.0, 0.0, 0.0].
        // CHV[k] = 0.0 (guard: SHL[k] == 0.0 -> 0.0).
        // 5 points -> 2 CHV values = 0.0 at points[3].ts and points[4].ts.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, 10.0, 10.0))
            .collect();
        let result = compute(&points, 2, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[1].value, 0.0);
        assert_eq!(vals[1].timestamp, 5);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[], 10, 10).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_smooth_plus_chv_minus_one_points_yields_empty_buffer() {
        // smooth_period=2, chv_period=3, 3 points -> SHL has 2 entries ->
        // 2 < 4 (chv + 1) -> no CHV values.
        let points: Vec<ResampledPointD1> = (0..3)
            .map(|i| rp(i + 1, (i + 1) as i64, 11.0, 9.0))
            .collect();
        let result = compute(&points, 2, 3).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_smooth_plus_chv_minus_one_points_yields_empty_buffer() {
        // smooth_period=2, chv_period=3, 4 points (= smooth + chv - 1) ->
        // SHL has 3 entries -> 3 < 4 (chv + 1) -> no CHV values.
        let points: Vec<ResampledPointD1> = (0..4)
            .map(|i| rp(i + 1, (i + 1) as i64, 11.0, 9.0))
            .collect();
        let result = compute(&points, 2, 3).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_smooth_plus_chv_points_yields_single_value() {
        // smooth_period=2, chv_period=3, 5 points (= smooth + chv) ->
        // SHL has 4 entries -> 1 CHV value = 0.0 (constant spread).
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 1) as i64, 11.0, 9.0))
            .collect();
        let result = compute(&points, 2, 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 5);
    }

    #[test]
    fn non_positive_smooth_period_returns_error() {
        let points = vec![rp(1, 1, 11.0, 9.0)];
        assert_eq!(
            compute(&points, 0, 10).unwrap_err(),
            IndicatorError::InvalidChvSmoothPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3, 10).unwrap_err(),
            IndicatorError::InvalidChvSmoothPeriod { n: -3 }
        );
    }

    #[test]
    fn non_positive_chv_period_returns_error() {
        let points = vec![rp(1, 1, 11.0, 9.0)];
        assert_eq!(
            compute(&points, 10, 0).unwrap_err(),
            IndicatorError::InvalidChvPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, 10, -5).unwrap_err(),
            IndicatorError::InvalidChvPeriod { n: -5 }
        );
    }

    #[test]
    fn timestamps_track_evaluated_bar() {
        // smooth_period=2, chv_period=2, 5 points -> SHL has 4 entries ->
        // 2 CHV values at original indices 3 and 4 -> timestamps points[3].ts,
        // points[4].ts.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, (i + 100) as i64, (10 + i) as f64, 10.0))
            .collect();
        let result = compute(&points, 2, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].timestamp, 103);
        assert_eq!(vals[1].timestamp, 104);
    }

    // --- Level 2: real CHV output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from the real EURUSD development DB.
    ///
    /// These values are **not ground-truth** — they were produced by running
    /// `chv::compute` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `smooth_period = 10` and `chv_period = 10` (the MQL5 defaults) and
    /// capturing the output. They serve as a regression baseline. The user
    /// will later compare them against an external source of truth.
    #[test]
    fn level2_real_chv_output_matches_captured_baseline() {
        let points = match test_support::fetch_eurusd_points() {
            Some(p) => p,
            None => {
                eprintln!("skipping level2_baseline_test: development DB unavailable");
                return;
            }
        };
        let result = compute(&points, 10, 10).unwrap();
        let vals = indicator_values!(result);

        // 2400 points -> 2381 CHV values (2400 - 10 - 10 + 1).
        assert_eq!(vals.len(), 2381);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1356998400);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1356998400, -15.088865149326457),
            (1357084800, 1.2100317907596583),
            (1357171200, 10.30911701184556),
            (1357257600, 7.398572894612063),
            (1357516800, 10.462477354102248),
            (1357603200, 13.348004055311403),
            (1357689600, 8.615345136290397),
            (1357776000, 42.238898526031264),
            (1357862400, 39.44359687729472),
            (1358121600, 35.03352419474165),
            (1358208000, 60.69616756027985),
            (1358294400, 25.15747292865511),
            (1358380800, 8.696212327690102),
            (1358467200, 12.343996055222767),
            (1358726400, -4.647604154590855),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
