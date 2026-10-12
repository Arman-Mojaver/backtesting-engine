use crate::domain::cast;
use crate::domain::indicators::moving_averages::{ema, sma};
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorSeries};
use crate::empty_oscillator;
use crate::validate_period;

/// Moving-average method for a MACD line, parsed from the database
/// `fast_type` / `slow_type` parameter values.
#[derive(Clone, Copy)]
enum MaMethod {
    Ema,
    Sma,
}

/// Parses a MACD moving-average method name. Only `"ema"` and `"sma"` exist
/// in the database; anything else is an error.
fn parse_ma_method(ma_type: &str) -> Result<MaMethod, IndicatorError> {
    match ma_type {
        "ema" => Ok(MaMethod::Ema),
        "sma" => Ok(MaMethod::Sma),
        _ => Err(IndicatorError::InvalidMacdMaType {
            ma_type: ma_type.to_string(),
        }),
    }
}

/// Computes the moving average with the given method. Both helpers share the
/// same index convention: entry `j` corresponds to original series index
/// `period - 1 + j`.
fn moving_average(values: &[f64], method: MaMethod, period: usize) -> Vec<f64> {
    match method {
        MaMethod::Ema => ema(values, period),
        MaMethod::Sma => sma(values, period),
    }
}

/// Compute the MACD main line from resampled daily points.
///
/// `MACD[i] = MA(close, fast) - MA(close, slow)` at every bar where both
/// moving averages exist (MQL5 `MACD.mq5`:
/// `ExtMacdBuffer[i] = ExtFastMaBuffer[i] - ExtSlowMaBuffer[i]` on
/// `PRICE_CLOSE`). The MQL5 example hardcodes `MODE_EMA` for both lines; the
/// database generalises it with a per-line `fast_type` / `slow_type`
/// (`"ema"` or `"sma"`), so each line uses its own method.
///
/// Both moving-average helpers share the index convention (entry `j`
/// corresponds to original index `period - 1 + j`), so the first MACD value
/// sits at index `max(fast, slow) - 1`. Unlike the CHO/OsMA modules, no
/// fast/slow order is enforced: the database deliberately contains
/// `fast >= slow` rows, and `MA(fast) - MA(slow)` is well-defined for any
/// order. Each value is tagged with the timestamp of the resampled point at
/// the evaluated index.
///
/// Insufficient data (fewer than `max(fast, slow)` points) yields an empty
/// `items` vector (not an error), per umbrella PRD Decision 10. A non-positive
/// `fast_n` or `slow_n`, or a method other than `"ema"` / `"sma"`, is an
/// error.
pub fn compute(
    points: &[ResampledPointD1],
    fast_type: &str,
    fast_n: i32,
    slow_type: &str,
    slow_n: i32,
) -> Result<OscillatorSeries, IndicatorError> {
    validate_period!(fast_n, InvalidMacdFastPeriod; slow_n, InvalidMacdSlowPeriod);
    let fast_method = parse_ma_method(fast_type)?;
    let slow_method = parse_ma_method(slow_type)?;
    let fast = cast::i32_to_usize(fast_n);
    let slow = cast::i32_to_usize(slow_n);

    let closes: Vec<f64> = points.iter().map(|pt| pt.close).collect();
    let fast_ma = moving_average(&closes, fast_method, fast);
    let slow_ma = moving_average(&closes, slow_method, slow);

    if fast_ma.is_empty() || slow_ma.is_empty() {
        empty_oscillator!();
    }

    // Entry `j` of an MA with period `p` corresponds to original index
    // `p - 1 + j`, so MACD is emitted at every original index from
    // `max(fast, slow) - 1`: the shorter-period MA is skipped ahead by the
    // period difference to stay aligned with the longer-period one.
    let items = if fast <= slow {
        // Limiting buffer: slow MA (entries map to indices `slow - 1 + j`).
        let delta = slow.saturating_sub(fast);
        slow_ma
            .iter()
            .zip(fast_ma.iter().skip(delta))
            .zip(points.iter().skip(slow.saturating_sub(1)))
            .map(|((slow_val, fast_val), point)| IndicatorValue {
                timestamp: point.timestamp,
                value: fast_val - slow_val,
            })
            .collect()
    } else {
        // Limiting buffer: fast MA (entries map to indices `fast - 1 + j`).
        let delta = fast.saturating_sub(slow);
        fast_ma
            .iter()
            .zip(slow_ma.iter().skip(delta))
            .zip(points.iter().skip(fast.saturating_sub(1)))
            .map(|((fast_val, slow_val), point)| IndicatorValue {
                timestamp: point.timestamp,
                value: fast_val - slow_val,
            })
            .collect()
    };

    Ok(OscillatorSeries { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};
    use crate::indicator_values;

    /// Build a resampled point with an explicit close (the only component the
    /// MACD computation consumes).
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

    fn points_from_closes(closes: &[f64]) -> Vec<ResampledPointD1> {
        closes
            .iter()
            .enumerate()
            .map(|(i, close)| {
                rp(
                    i32::try_from(i).unwrap().saturating_add(1),
                    i64::try_from(i).unwrap().saturating_add(1),
                    *close,
                )
            })
            .collect()
    }

    // --- synthetic hand-computed tests (all four database type combos) ---

    #[test]
    fn exponential_close_ema_ema_yields_hand_computed_macd() {
        // close = [3, 6, 12, 24, 48, 96], fast=1 ema, slow=3 ema, N=6.
        // FastEMA(1) = identity = [3, 6, 12, 24, 48, 96] (len 6).
        // SlowEMA(3): seed=SMA(3,6,12)=7.0; [7.0, 15.5, 31.75, 63.875] (len 4).
        // delta = 2. MACD[j] = FastEMA[2+j] - SlowEMA[j]:
        //   MACD = [12-7.0, 24-15.5, 48-31.75, 96-63.875]
        //        = [5.0, 8.5, 16.25, 32.125] (len 4).
        // Values at original indices 2..5 -> ts 3..6.
        let points = points_from_closes(&[3.0, 6.0, 12.0, 24.0, 48.0, 96.0]);
        let result = compute(&points, "ema", 1, "ema", 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 5.0);
        assert_eq!(vals[0].timestamp, 3);
        assert_eq!(vals[1].value, 8.5);
        assert_eq!(vals[1].timestamp, 4);
        assert_eq!(vals[2].value, 16.25);
        assert_eq!(vals[2].timestamp, 5);
        assert_eq!(vals[3].value, 32.125);
        assert_eq!(vals[3].timestamp, 6);
    }

    #[test]
    fn ramp_close_sma_sma_yields_hand_computed_macd() {
        // close = [1, 2, 3, 4, 5, 6, 7], fast=2 sma, slow=4 sma, N=7.
        // FastSMA(2) = [1.5, 2.5, 3.5, 4.5, 5.5, 6.5] (len 6).
        // SlowSMA(4) = [2.5, 3.5, 4.5, 5.5] (len 4).
        // delta = 2. MACD[j] = FastSMA[2+j] - SlowSMA[j] = 1.0 for all j.
        // Values at original indices 3..6 -> ts 4..7.
        let points = points_from_closes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        let result = compute(&points, "sma", 2, "sma", 4).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        for (i, val) in vals.iter().enumerate() {
            assert_eq!(val.value, 1.0, "value mismatch at index {i}");
            assert_eq!(
                val.timestamp,
                i64::try_from(i).unwrap().saturating_add(4),
                "ts mismatch at {i}"
            );
        }
    }

    #[test]
    fn exponential_close_sma_fast_ema_slow_yields_hand_computed_macd() {
        // close = [3, 6, 12, 24, 48, 96], fast=2 sma, slow=3 ema, N=6.
        // FastSMA(2) = [4.5, 9.0, 18.0, 36.0, 72.0] (len 5).
        // SlowEMA(3) = [7.0, 15.5, 31.75, 63.875] (len 4).
        // delta = 1. MACD[j] = FastSMA[1+j] - SlowEMA[j]:
        //   MACD = [9-7.0, 18-15.5, 36-31.75, 72-63.875]
        //        = [2.0, 2.5, 4.25, 8.125] (len 4).
        // Values at original indices 2..5 -> ts 3..6.
        let points = points_from_closes(&[3.0, 6.0, 12.0, 24.0, 48.0, 96.0]);
        let result = compute(&points, "sma", 2, "ema", 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 2.0);
        assert_eq!(vals[0].timestamp, 3);
        assert_eq!(vals[1].value, 2.5);
        assert_eq!(vals[1].timestamp, 4);
        assert_eq!(vals[2].value, 4.25);
        assert_eq!(vals[2].timestamp, 5);
        assert_eq!(vals[3].value, 8.125);
        assert_eq!(vals[3].timestamp, 6);
    }

    #[test]
    fn fast_greater_than_slow_is_computed_not_rejected() {
        // close = [1, 2, 3, 4, 5, 6, 7], fast=4 sma, slow=2 sma, N=7
        // (fast > slow, as the database deliberately contains).
        // FastSMA(4) = [2.5, 3.5, 4.5, 5.5] (len 4).
        // SlowSMA(2) = [1.5, 2.5, 3.5, 4.5, 5.5, 6.5] (len 6).
        // Branch fast > slow: delta = 2, MACD[j] = FastSMA[j] - SlowSMA[2+j]:
        //   MACD = [2.5-3.5, 3.5-4.5, 4.5-5.5, 5.5-6.5] = [-1.0; 4].
        // Values at original indices 3..6 -> ts 4..7.
        let points = points_from_closes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        let result = compute(&points, "sma", 4, "sma", 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        for (i, val) in vals.iter().enumerate() {
            assert_eq!(val.value, -1.0, "value mismatch at index {i}");
            assert_eq!(
                val.timestamp,
                i64::try_from(i).unwrap().saturating_add(4),
                "ts mismatch at {i}"
            );
        }
    }

    #[test]
    fn equal_periods_same_type_yield_zero_macd() {
        // close = [3, 6, 12, 24, 48, 96], fast=3 ema, slow=3 ema.
        // Identical lines -> MACD identically 0.0 at indices 2..5.
        let points = points_from_closes(&[3.0, 6.0, 12.0, 24.0, 48.0, 96.0]);
        let result = compute(&points, "ema", 3, "ema", 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert!(vals.iter().all(|v| v.value == 0.0));
    }

    #[test]
    fn equal_periods_mixed_types_yield_nonzero_macd() {
        // close = [3, 6, 12, 24, 48, 96], fast=3 ema, slow=3 sma.
        // FastEMA(3) = [7.0, 15.5, 31.75, 63.875].
        // SlowSMA(3) = [7.0, 14.0, 28.0, 56.0].
        // MACD = [0.0, 1.5, 3.75, 7.875] at indices 2..5 -> ts 3..6.
        let points = points_from_closes(&[3.0, 6.0, 12.0, 24.0, 48.0, 96.0]);
        let result = compute(&points, "ema", 3, "sma", 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[1].value, 1.5);
        assert_eq!(vals[2].value, 3.75);
        assert_eq!(vals[3].value, 7.875);
    }

    #[test]
    fn constant_close_yields_zero_macd() {
        // 8 constant points, fast=3 ema, slow=5 sma -> 4 zero values
        // (len = 8 - 5 + 1) at indices 4..7 -> ts 5..8.
        let points = points_from_closes(&[10.0; 8]);
        let result = compute(&points, "ema", 3, "sma", 5).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert!(vals.iter().all(|v| v.value == 0.0));
        assert_eq!(vals[0].timestamp, 5);
        assert_eq!(vals[3].timestamp, 8);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[], "ema", 12, "ema", 26).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_max_period_points_yields_empty_buffer() {
        // max(fast, slow) = 5; 4 points -> slow MA empty -> no MACD values.
        let points = points_from_closes(&[10.0; 4]);
        let result = compute(&points, "ema", 3, "sma", 5).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_max_period_points_yields_single_value() {
        // max(fast, slow) = 5; 5 points -> 1 MACD value (constant -> 0.0).
        let points = points_from_closes(&[10.0; 5]);
        let result = compute(&points, "ema", 3, "sma", 5).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 5);
    }

    #[test]
    fn timestamps_track_evaluated_bar() {
        // fast=3, slow=5 -> values start at index 4.
        // 8 points with ts 100..107 -> values at ts 104..107.
        let points: Vec<ResampledPointD1> = (0..8)
            .map(|i: i32| rp(i.saturating_add(1), i64::from(i.saturating_add(100)), 10.0))
            .collect();
        let result = compute(&points, "ema", 3, "sma", 5).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].timestamp, 104);
        assert_eq!(vals[1].timestamp, 105);
        assert_eq!(vals[2].timestamp, 106);
        assert_eq!(vals[3].timestamp, 107);
    }

    #[test]
    fn output_length_is_n_minus_max_period_plus_one() {
        // 10 points, fast=3, slow=5 -> 10 - 5 + 1 = 6 MACD values.
        let points = points_from_closes(&[10.0; 10]);
        let result = compute(&points, "ema", 3, "ema", 5).unwrap();
        assert_eq!(indicator_values!(result).len(), 6);
    }

    #[test]
    fn non_positive_fast_period_returns_error() {
        let points = vec![rp(1, 1, 10.0)];
        assert_eq!(
            compute(&points, "ema", 0, "ema", 26).unwrap_err(),
            IndicatorError::InvalidMacdFastPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, "ema", -3, "ema", 26).unwrap_err(),
            IndicatorError::InvalidMacdFastPeriod { n: -3 }
        );
    }

    #[test]
    fn non_positive_slow_period_returns_error() {
        let points = vec![rp(1, 1, 10.0)];
        assert_eq!(
            compute(&points, "ema", 12, "ema", 0).unwrap_err(),
            IndicatorError::InvalidMacdSlowPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, "ema", 12, "ema", -5).unwrap_err(),
            IndicatorError::InvalidMacdSlowPeriod { n: -5 }
        );
    }

    #[test]
    fn unknown_ma_type_returns_error() {
        let points = vec![rp(1, 1, 10.0)];
        assert_eq!(
            compute(&points, "wma", 12, "ema", 26).unwrap_err(),
            IndicatorError::InvalidMacdMaType {
                ma_type: "wma".to_string()
            }
        );
        assert_eq!(
            compute(&points, "ema", 12, "lwma", 26).unwrap_err(),
            IndicatorError::InvalidMacdMaType {
                ma_type: "lwma".to_string()
            }
        );
    }

    // --- Level 2: real MACD output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from real EURUSD data (frozen fixture in `test_support`).
    ///
    /// These values are **not ground-truth** — they were produced by running
    /// `macd::compute` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `fast = 12`, `slow = 20`, both `"ema"` (a database-plausible
    /// configuration) and capturing the output. They serve as a regression
    /// baseline. The user will later compare them against an external source
    /// of truth.
    #[test]
    fn level2_real_macd_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, "ema", 12, "ema", 20).unwrap();
        let vals = indicator_values!(result);

        // 2400 points -> 2381 MACD values (2400 - 20 + 1).
        assert_eq!(vals.len(), 2381);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1356998400);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1356998400, 0.0045443209957420905),
            (1357084800, 0.004197897985408039),
            (1357171200, 0.0029791653806408025),
            (1357257600, 0.002184763271047885),
            (1357516800, 0.0018815797765820985),
            (1357603200, 0.0013581670679752023),
            (1357689600, 0.0007840061728443537),
            (1357776000, 0.001580139316530449),
            (1357862400, 0.002604869242406771),
            (1358121600, 0.0035557449843062194),
            (1358208000, 0.0037668440689939686),
            (1358294400, 0.003831651147469195),
            (1358380800, 0.0043186038024563),
            (1358467200, 0.004306950651109975),
            (1358726400, 0.004210308485411041),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
