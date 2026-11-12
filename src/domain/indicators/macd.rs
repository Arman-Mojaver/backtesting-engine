//! MACD: the MACD line crossing its signal line.
//!
//! `MACD[i] = MA(close, fast) - MA(close, slow)`, each line `ema` or `sma`;
//! the signal line is `SMA(MACD, SIGNAL_N)`. The output is two crossover buffers:
//! signals fire where the MACD line crosses its signal line. The MQL5
//! example (`MACD.mq5`) hardcodes `MODE_EMA` for both lines and
//! `InpSignalSMA = 9`; the database generalises the lines with a per-line
//! type but carries no signal period, so the engine fixes it at the MQL5
//! default.
use crate::domain::cast;
use crate::domain::indicator::MovingAverageType;
use crate::domain::indicators::moving_averages::{ema, sma};
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{CrossoverBuffers, IndicatorValue};
use crate::empty_crossover;
use crate::validate_period;

/// Signal-line period, fixed at the MQL5 reference default
/// (`MACD.mq5`: `InpSignalSMA = 9`). The database stores no MACD signal
/// period (`MacdParametersSchema` is slow/fast only), so the engine
/// carries the reference default as a constant.
const SIGNAL_N: i32 = 9;

fn parse_ma_method(ma_type: &str) -> Result<MovingAverageType, IndicatorError> {
    match ma_type {
        "ema" => Ok(MovingAverageType::Ema),
        "sma" => Ok(MovingAverageType::Sma),
        _ => Err(IndicatorError::InvalidMacdMaType {
            ma_type: ma_type.to_string(),
        }),
    }
}

fn moving_average(values: &[f64], ma_type: MovingAverageType, period: usize) -> Vec<f64> {
    match ma_type {
        MovingAverageType::Ema => ema(values, period),
        MovingAverageType::Sma => sma(values, period),
    }
}

/// The MACD line: `MA(close, fast) - MA(close, slow)`, one value per bar
/// from the slow line's first value onward. Empty when the slow line is
/// empty (not enough data), never an error for short input.
fn macd_line(
    points: &[ResampledPointD1],
    fast_type: &str,
    fast_n: i32,
    slow_type: &str,
    slow_n: i32,
) -> Result<Vec<IndicatorValue>, IndicatorError> {
    validate_period!(fast_n, InvalidMacdFastPeriod; slow_n, InvalidMacdSlowPeriod);

    if fast_n >= slow_n {
        return Err(IndicatorError::InvalidMacdPeriodOrder {
            fast: fast_n,
            slow: slow_n,
        });
    }
    let fast_method = parse_ma_method(fast_type)?;
    let slow_method = parse_ma_method(slow_type)?;
    let fast = cast::i32_to_usize(fast_n);
    let slow = cast::i32_to_usize(slow_n);

    let closes: Vec<f64> = points.iter().map(|pt| pt.close).collect();
    let fast_ma = moving_average(&closes, fast_method, fast);
    let slow_ma = moving_average(&closes, slow_method, slow);

    if fast_ma.is_empty() || slow_ma.is_empty() {
        return Ok(Vec::new());
    }

    let delta = slow.saturating_sub(fast);
    Ok(slow_ma
        .iter()
        .zip(fast_ma.iter().skip(delta))
        .zip(points.iter().skip(slow.saturating_sub(1)))
        .map(|((slow_val, fast_val), point)| IndicatorValue {
            timestamp: point.timestamp,
            value: fast_val - slow_val,
        })
        .collect())
}

/// Crosses the MACD line against its signal line, as crossover buffers.
///
/// The long buffer is the MACD line; the short buffer is the signal line,
/// `SMA(MACD, SIGNAL_N)`. Both buffers share the timestamps of the bars where
/// both lines exist: the first `SIGNAL_N - 1` MACD values, whose signal average
/// does not yet exist, are dropped so the buffers stay zip-aligned. Short input
/// yields empty buffers, never an error.
pub fn compute(
    points: &[ResampledPointD1],
    fast_type: &str,
    fast_n: i32,
    slow_type: &str,
    slow_n: i32,
) -> Result<CrossoverBuffers, IndicatorError> {
    let line = macd_line(points, fast_type, fast_n, slow_type, slow_n)?;
    let values: Vec<f64> = line.iter().map(|value| value.value).collect();
    let signal = sma(&values, cast::i32_to_usize(SIGNAL_N));

    if signal.is_empty() {
        empty_crossover!();
    }

    // The signal window ending at line[j + SIGNAL_N - 1] gives the signal
    // line its first value there, so both buffers start where the signal
    // line does and share the MACD line's timestamps.
    let long_buffer: Vec<IndicatorValue> = line
        .into_iter()
        .skip(cast::i32_to_usize(SIGNAL_N.saturating_sub(1)))
        .collect();
    let short_buffer: Vec<IndicatorValue> = signal
        .into_iter()
        .zip(long_buffer.iter())
        .map(|(value, bar)| IndicatorValue {
            timestamp: bar.timestamp,
            value,
        })
        .collect();

    Ok(CrossoverBuffers {
        upcross_long_buffer: long_buffer,
        upcross_short_buffer: short_buffer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::ResampledPointD1;

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

    // --- synthetic hand-computed tests of the MACD line (all database type combos) ---

    #[test]
    fn exponential_close_ema_ema_yields_hand_computed_macd_line() {
        // close = [3, 6, 12, 24, 48, 96], fast=1 ema, slow=3 ema, N=6.
        // FastEMA(1) = identity = [3, 6, 12, 24, 48, 96] (len 6).
        // SlowEMA(3): seed=SMA(3,6,12)=7.0; [7.0, 15.5, 31.75, 63.875] (len 4).
        // delta = 2. MACD[j] = FastEMA[2+j] - SlowEMA[j]:
        //   MACD = [12-7.0, 24-15.5, 48-31.75, 96-63.875]
        //        = [5.0, 8.5, 16.25, 32.125] (len 4).
        // Values at original indices 2..5 -> ts 3..6.
        let points = points_from_closes(&[3.0, 6.0, 12.0, 24.0, 48.0, 96.0]);
        let vals = macd_line(&points, "ema", 1, "ema", 3).unwrap();
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
    fn ramp_close_sma_sma_yields_hand_computed_macd_line() {
        // close = [1, 2, 3, 4, 5, 6, 7], fast=2 sma, slow=4 sma, N=7.
        // FastSMA(2) = [1.5, 2.5, 3.5, 4.5, 5.5, 6.5] (len 6).
        // SlowSMA(4) = [2.5, 3.5, 4.5, 5.5] (len 4).
        // delta = 2. MACD[j] = FastSMA[2+j] - SlowSMA[j] = 1.0 for all j.
        // Values at original indices 3..6 -> ts 4..7.
        let points = points_from_closes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        let vals = macd_line(&points, "sma", 2, "sma", 4).unwrap();
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
    fn exponential_close_sma_fast_ema_slow_yields_hand_computed_macd_line() {
        // close = [3, 6, 12, 24, 48, 96], fast=2 sma, slow=3 ema, N=6.
        // FastSMA(2) = [4.5, 9.0, 18.0, 36.0, 72.0] (len 5).
        // SlowEMA(3) = [7.0, 15.5, 31.75, 63.875] (len 4).
        // delta = 1. MACD[j] = FastSMA[1+j] - SlowEMA[j]:
        //   MACD = [9-7.0, 18-15.5, 36-31.75, 72-63.875]
        //        = [2.0, 2.5, 4.25, 8.125] (len 4).
        // Values at original indices 2..5 -> ts 3..6.
        let points = points_from_closes(&[3.0, 6.0, 12.0, 24.0, 48.0, 96.0]);
        let vals = macd_line(&points, "sma", 2, "ema", 3).unwrap();
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
    fn fast_greater_than_slow_is_rejected() {
        // fast=4 > slow=2 must be rejected with InvalidMacdPeriodOrder,
        // through the public entry point.
        let points = points_from_closes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]);
        let err = compute(&points, "sma", 4, "sma", 2).unwrap_err();
        assert_eq!(
            err,
            IndicatorError::InvalidMacdPeriodOrder { fast: 4, slow: 2 },
        );
    }

    #[test]
    fn equal_period_gap_same_type_yields_zero_macd_line() {
        // close = [10, 10, 10, 10, 10, 10], fast=3 ema, slow=5 ema (fast < slow).
        // Both EMAs on constant data converge to 10.0 -> MACD identically 0.0.
        let points = points_from_closes(&[10.0; 6]);
        let vals = macd_line(&points, "ema", 3, "ema", 5).unwrap();
        assert_eq!(vals.len(), 2);
        assert!(vals.iter().all(|v| v.value == 0.0));
    }

    #[test]
    fn equal_period_gap_mixed_types_yield_nonzero_macd_line() {
        // close = [3, 6, 12, 24, 48, 96], fast=3 ema, slow=5 sma (fast < slow).
        // Different periods and types produce nonzero MACD.
        let points = points_from_closes(&[3.0, 6.0, 12.0, 24.0, 48.0, 96.0]);
        let vals = macd_line(&points, "ema", 3, "sma", 5).unwrap();
        assert_eq!(vals.len(), 2);
        assert!(!vals.iter().all(|v| v.value == 0.0));
    }

    #[test]
    fn constant_close_yields_zero_macd_line() {
        // 8 constant points, fast=3 ema, slow=5 sma -> 4 zero values
        // (len = 8 - 5 + 1) at indices 4..7 -> ts 5..8.
        let points = points_from_closes(&[10.0; 8]);
        let vals = macd_line(&points, "ema", 3, "sma", 5).unwrap();
        assert_eq!(vals.len(), 4);
        assert!(vals.iter().all(|v| v.value == 0.0));
        assert_eq!(vals[0].timestamp, 5);
        assert_eq!(vals[3].timestamp, 8);
    }

    // --- edge cases of the MACD line ---

    #[test]
    fn empty_points_yields_empty_macd_line() {
        let vals = macd_line(&[], "ema", 12, "ema", 26).unwrap();
        assert!(vals.is_empty());
    }

    #[test]
    fn fewer_than_max_period_points_yields_empty_macd_line() {
        // max(fast, slow) = 5; 4 points -> slow MA empty -> no MACD values.
        let points = points_from_closes(&[10.0; 4]);
        let vals = macd_line(&points, "ema", 3, "sma", 5).unwrap();
        assert!(vals.is_empty());
    }

    #[test]
    fn exactly_max_period_points_yields_single_macd_line_value() {
        // max(fast, slow) = 5; 5 points -> 1 MACD value (constant -> 0.0).
        let points = points_from_closes(&[10.0; 5]);
        let vals = macd_line(&points, "ema", 3, "sma", 5).unwrap();
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 5);
    }

    #[test]
    fn macd_line_timestamps_track_evaluated_bar() {
        // fast=3, slow=5 -> values start at index 4.
        // 8 points with ts 100..107 -> values at ts 104..107.
        let points: Vec<ResampledPointD1> = (0..8)
            .map(|i: i32| rp(i.saturating_add(1), i64::from(i.saturating_add(100)), 10.0))
            .collect();
        let vals = macd_line(&points, "ema", 3, "sma", 5).unwrap();
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].timestamp, 104);
        assert_eq!(vals[1].timestamp, 105);
        assert_eq!(vals[2].timestamp, 106);
        assert_eq!(vals[3].timestamp, 107);
    }

    #[test]
    fn macd_line_length_is_n_minus_slow_plus_one() {
        // 10 points, fast=3, slow=5 -> 10 - 5 + 1 = 6 MACD values.
        let points = points_from_closes(&[10.0; 10]);
        let vals = macd_line(&points, "ema", 3, "ema", 5).unwrap();
        assert_eq!(vals.len(), 6);
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

    // --- crossover buffers: the MACD line against its signal line ---

    #[test]
    fn empty_points_yield_empty_pair() {
        let buffers = compute(&[], "ema", 12, "ema", 26).unwrap();
        assert!(buffers.upcross_long_buffer.is_empty());
        assert!(buffers.upcross_short_buffer.is_empty());
    }

    #[test]
    fn line_shorter_than_signal_window_yields_empty_pair() {
        // 12 points, fast=3, slow=5 -> line has 12 - 5 + 1 = 8 values,
        // shorter than the 9-bar signal window -> no signal line -> empty buffers.
        let points = points_from_closes(&[10.0; 12]);
        let buffers = compute(&points, "ema", 3, "sma", 5).unwrap();
        assert!(buffers.upcross_long_buffer.is_empty());
        assert!(buffers.upcross_short_buffer.is_empty());
    }

    #[test]
    fn crossover_pair_aligns_line_with_signal_line() {
        // fast=1 ema (identity), slow=2 sma: line[j] = (close[j+1] - close[j]) / 2,
        // evaluated at point j+1. closes drop 100 -> 82 at index 9, recover at 10.
        //   line = [0,0,0,0,0,0,0,0,-9,9,0,0,0] (len 13) at ts 2000..14000.
        //   signal = SMA(line, 9): [-1, 0, 0, 0, 0].
        // Buffers start at line index 8 (ts 10000):
        //   long  = [-9, 9, 0, 0, 0]
        //   short = [-1, 0, 0, 0, 0]
        let closes = [
            100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 100.0, 82.0, 100.0, 100.0,
            100.0, 100.0,
        ];
        let points: Vec<ResampledPointD1> = (0..closes.len())
            .map(|i| {
                rp(
                    i32::try_from(i).unwrap().saturating_add(1),
                    i64::try_from(i)
                        .unwrap()
                        .saturating_add(1)
                        .saturating_mul(1000),
                    closes[i],
                )
            })
            .collect();
        let buffers = compute(&points, "ema", 1, "sma", 2).unwrap();

        let expected_ts = [10000_i64, 11000, 12000, 13000, 14000];
        let long = &buffers.upcross_long_buffer;
        let short = &buffers.upcross_short_buffer;
        assert_eq!(long.len(), 5);
        assert_eq!(short.len(), 5);
        for (i, ts) in expected_ts.iter().enumerate() {
            assert_eq!(long[i].timestamp, *ts, "long ts mismatch at {i}");
            assert_eq!(short[i].timestamp, *ts, "short ts mismatch at {i}");
        }
        let long_vals: Vec<f64> = long.iter().map(|v| v.value).collect();
        let short_vals: Vec<f64> = short.iter().map(|v| v.value).collect();
        assert_eq!(long_vals, vec![-9.0, 9.0, 0.0, 0.0, 0.0]);
        assert_eq!(short_vals, vec![-1.0, 0.0, 0.0, 0.0, 0.0]);
    }

    // --- Level 2: real MACD output on real EURUSD data (captured baselines) ---

    /// Baseline captured from real EURUSD data (frozen fixture in `test_support`).
    ///
    /// These values are **not ground-truth**; they were produced by running
    /// `macd_line` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `fast = 12`, `slow = 20`, both `"ema"` (a database-plausible
    /// configuration) and capturing the output. They serve as a regression
    /// baseline. The user will later compare them against an external source
    /// of truth.
    #[test]
    fn level2_real_macd_line_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let vals = macd_line(&points, "ema", 12, "ema", 20).unwrap();

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

    /// Captured-buffers baseline (see the line baseline above for the caveat).
    ///
    /// The long buffer is the MACD line from index `SIGNAL_N - 1` on (its
    /// first 8 values have no signal average yet); the short buffer is the
    /// SMA(9) signal line, both captured from the same real-data run.
    #[test]
    fn level2_real_macd_pair_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let buffers = compute(&points, "ema", 12, "ema", 20).unwrap();
        let long = &buffers.upcross_long_buffer;
        let short = &buffers.upcross_short_buffer;

        // 2400 points -> line 2381, signal window 9 -> 2373 aligned values.
        assert_eq!(long.len(), 2373);
        assert_eq!(short.len(), 2373);

        // Buffers start 8 bars after the line's first bar (line index 8).
        assert_eq!(long[0].timestamp, 1357862400);
        assert_eq!(short[0].timestamp, 1357862400);
        assert_eq!(long.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(short.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, long, short) triples captured from the run.
        let expected: &[(i64, f64, f64)] = &[
            (1357862400, 0.002604869242406771, 0.0024572121343530767),
            (1358121600, 0.0035557449843062194, 0.0023473703553046465),
            (1358208000, 0.0037668440689939686, 0.0022994754757030833),
            (1358294400, 0.003831651147469195, 0.0023941961164617937),
            (1358380800, 0.0043186038024563, 0.0026312895088405064),
            (1358467200, 0.004306950651109975, 0.0029007751615658262),
            (1358726400, 0.004210308485411041, 0.0032176797635031416),
            (1358812800, 0.004109797692104289, 0.0035872121545320232),
            (1358899200, 0.0039028894629120625, 0.0038452955041299803),
            (1358985600, 0.004095214086528021, 0.004010889375699008),
            (1359072000, 0.004674170093815988, 0.004135158832311204),
            (1359331200, 0.004942247475310246, 0.004265759210790791),
            (1359417600, 0.005321218840991904, 0.004431266732293314),
            (1359504000, 0.006008105800931363, 0.004618989176568321),
            (1359590400, 0.006558460934350974, 0.0048691569858173205),
        ];
        for (i, (ts, l, s)) in expected.iter().enumerate() {
            assert_eq!(long[i].timestamp, *ts, "long ts mismatch at {i}");
            assert_eq!(long[i].value, *l, "long value mismatch at {i}");
            assert_eq!(short[i].value, *s, "short value mismatch at {i}");
        }
    }
}
