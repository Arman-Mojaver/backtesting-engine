use crate::domain::indicators::moving_averages::{ema, sma};
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorSeries};

/// Compute the OsMA (Moving Average of Oscillator, aka MACD histogram) from
/// resampled daily points.
///
/// `MACD = EMA(close, fast_ema) - EMA(close, slow_ema)` and
/// `Signal = SMA(MACD, signal_sma)`, then `OsMA[k] = MACD[k + signal_sma - 1]
/// - Signal[k]` (MQL5 `OsMA.mq5`, `MODE_EMA` on `PRICE_CLOSE` for fast/slow,
/// `MODE_SMA` via `SimpleMAOnBuffer` for the signal line).
///
/// The slow EMA is the limiting factor (by convention `slow_ema > fast_ema`).
/// The slow EMA seed is at index `slow_ema - 1`; MACD starts at the same
/// index. Signal starts at `slow_ema + signal_sma - 2`; OsMA starts at the
/// same index. The MQL5 source sets
/// `PLOT_DRAW_BEGIN = InpSlowEMAPeriod + InpSignalSMAPeriod - 2`. Each value
/// is tagged with the timestamp of the resampled point at the evaluated index.
///
/// Insufficient data (fewer than `slow_ema + signal_sma - 1` points, i.e.
/// `signal` is empty) yields an empty `items` vector (not an error), per
/// umbrella PRD Decision 10. A non-positive `fast_ema`, `slow_ema`, or
/// `signal_sma` is an error.
pub fn compute(
    points: &[ResampledPointD1],
    fast_ema: i32,
    slow_ema: i32,
    signal_sma: i32,
) -> Result<OscillatorSeries, IndicatorError> {
    if fast_ema <= 0 {
        return Err(IndicatorError::InvalidOsMaFastPeriod { n: fast_ema });
    }
    if slow_ema <= 0 {
        return Err(IndicatorError::InvalidOsMaSlowPeriod { n: slow_ema });
    }
    if signal_sma <= 0 {
        return Err(IndicatorError::InvalidOsMaSignalPeriod { n: signal_sma });
    }
    let fast = fast_ema as usize;
    let slow = slow_ema as usize;
    let signal = signal_sma as usize;

    let closes: Vec<f64> = points.iter().map(|pt| pt.close).collect();
    let fast_ema_result = ema(&closes, fast);
    let slow_ema_result = ema(&closes, slow);

    // Slow EMA entry `j_s` corresponds to original index `slow - 1 + j_s`.
    // MACD[j_s] = FastEMA[delta + j_s] - SlowEMA[j_s] where
    // `delta = slow - fast` (by convention `slow >= fast`).
    let delta = slow - fast;
    let macd: Vec<f64> = (0..slow_ema_result.len())
        .map(|j_s| fast_ema_result[delta + j_s] - slow_ema_result[j_s])
        .collect();

    let signal_line = sma(&macd, signal);

    if signal_line.is_empty() {
        return Ok(OscillatorSeries { items: Vec::new() });
    }

    // OsMA[k] = macd[k + signal - 1] - signal[k] for k = 0..(signal.len()).
    // OsMA entry `k` corresponds to original index `slow + signal - 2 + k`.
    let start = slow + signal - 2;
    let items = (0..signal_line.len())
        .map(|k| IndicatorValue {
            timestamp: points[start + k].timestamp,
            value: macd[k + signal - 1] - signal_line[k],
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

    /// Build a resampled point with an explicit close (the only component the
    /// OsMA computation consumes).
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

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_close_yields_zero_osma() {
        // All bars: close=10, fast=3, slow=5, signal=2, N=8.
        // FastEMA = EMA([10;8], 3) = [10.0;6] (length 6).
        // SlowEMA = EMA([10;8], 5) = [10.0;4] (length 4).
        // delta = 5 - 3 = 2. MACD[j_s] = FastEMA[2+j_s] - SlowEMA[j_s] = 0.0.
        // MACD = [0.0;4] (length 4).
        // Signal = SMA(MACD, 2) = [0.0;3] (length 3).
        // OsMA[k] = MACD[k+1] - Signal[k] = 0.0 for k=0,1,2.
        // 8 points -> 3 OsMA values = 0.0 at points[5].ts, points[6].ts,
        // points[7].ts.
        let points: Vec<ResampledPointD1> =
            (0..8).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 3, 5, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 6);
        assert_eq!(vals[1].value, 0.0);
        assert_eq!(vals[1].timestamp, 7);
        assert_eq!(vals[2].value, 0.0);
        assert_eq!(vals[2].timestamp, 8);
    }

    #[test]
    fn exponential_close_fast1_yields_hand_computed_osma() {
        // close = [3, 6, 12, 24, 48, 96], fast=1, slow=3, signal=2, N=6.
        // FastEMA = EMA(close, 1) = [3.0, 6.0, 12.0, 24.0, 48.0, 96.0] (len 6).
        // SlowEMA = EMA(close, 3): seed=SMA(3,6,12)=7.0 at index 2;
        //   SlowEMA[1]=(24-7.0)*0.5+7.0=15.5;
        //   SlowEMA[2]=(48-15.5)*0.5+15.5=31.75;
        //   SlowEMA[3]=(96-31.75)*0.5+31.75=63.875.
        // SlowEMA = [7.0, 15.5, 31.75, 63.875] (len 4).
        // delta = 3 - 1 = 2. MACD[j_s] = FastEMA[2+j_s] - SlowEMA[j_s]:
        //   MACD = [12-7.0, 24-15.5, 48-31.75, 96-63.875]
        //        = [5.0, 8.5, 16.25, 32.125] (len 4).
        // Signal = SMA(MACD, 2) = [6.75, 12.375, 24.1875] (len 3).
        // OsMA[k] = MACD[k+1] - Signal[k]:
        //   OsMA = [8.5-6.75, 16.25-12.375, 32.125-24.1875]
        //        = [1.75, 3.875, 7.9375] (len 3).
        // 6 points -> 3 OsMA values at points[3].ts, points[4].ts,
        // points[5].ts.
        let closes = [3.0, 6.0, 12.0, 24.0, 48.0, 96.0];
        let points: Vec<ResampledPointD1> = (0..6)
            .map(|i| rp(i + 1, (i + 1) as i64, closes[i as usize]))
            .collect();
        let result = compute(&points, 1, 3, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].value, 1.75);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[1].value, 3.875);
        assert_eq!(vals[1].timestamp, 5);
        assert_eq!(vals[2].value, 7.9375);
        assert_eq!(vals[2].timestamp, 6);
    }

    #[test]
    fn reversing_close_fast1_yields_hand_computed_osma_with_negatives() {
        // close = [3, 6, 12, 24, 12, 6], fast=1, slow=3, signal=2, N=6.
        // FastEMA = [3.0, 6.0, 12.0, 24.0, 12.0, 6.0] (len 6).
        // SlowEMA = EMA(close, 3): seed=7.0;
        //   SlowEMA[1]=(24-7.0)*0.5+7.0=15.5;
        //   SlowEMA[2]=(12-15.5)*0.5+15.5=13.75;
        //   SlowEMA[3]=(6-13.75)*0.5+13.75=9.875.
        // SlowEMA = [7.0, 15.5, 13.75, 9.875] (len 4).
        // delta = 2. MACD = [12-7.0, 24-15.5, 12-13.75, 6-9.875]
        //   = [5.0, 8.5, -1.75, -3.875] (len 4).
        // Signal = SMA(MACD, 2) = [6.75, 3.375, -2.8125] (len 3).
        // OsMA = [8.5-6.75, -1.75-3.375, -3.875-(-2.8125)]
        //   = [1.75, -5.125, -1.0625] (len 3).
        // 6 points -> 3 OsMA values at points[3].ts, points[4].ts,
        // points[5].ts.
        let closes = [3.0, 6.0, 12.0, 24.0, 12.0, 6.0];
        let points: Vec<ResampledPointD1> = (0..6)
            .map(|i| rp(i + 1, (i + 1) as i64, closes[i as usize]))
            .collect();
        let result = compute(&points, 1, 3, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].value, 1.75);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[1].value, -5.125);
        assert_eq!(vals[1].timestamp, 5);
        assert_eq!(vals[2].value, -1.0625);
        assert_eq!(vals[2].timestamp, 6);
    }

    #[test]
    fn extended_exponential_close_fast1_yields_hand_computed_osma() {
        // close = [3, 6, 12, 24, 48, 96, 192], fast=1, slow=3, signal=2, N=7.
        // FastEMA = [3.0, 6.0, 12.0, 24.0, 48.0, 96.0, 192.0] (len 7).
        // SlowEMA = EMA(close, 3): seed=7.0; SlowEMA[1]=15.5;
        //   SlowEMA[2]=31.75; SlowEMA[3]=63.875;
        //   SlowEMA[4]=(192-63.875)*0.5+63.875=127.9375.
        // SlowEMA = [7.0, 15.5, 31.75, 63.875, 127.9375] (len 5).
        // delta = 2. MACD = [5.0, 8.5, 16.25, 32.125, 64.0625] (len 5).
        // Signal = SMA(MACD, 2) = [6.75, 12.375, 24.1875, 48.09375] (len 4).
        // OsMA = [1.75, 3.875, 7.9375, 15.96875] (len 4).
        // 7 points -> 4 OsMA values at points[3].ts, points[4].ts,
        // points[5].ts, points[6].ts.
        let closes = [3.0, 6.0, 12.0, 24.0, 48.0, 96.0, 192.0];
        let points: Vec<ResampledPointD1> = (0..7)
            .map(|i| rp(i + 1, (i + 1) as i64, closes[i as usize]))
            .collect();
        let result = compute(&points, 1, 3, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0].value, 1.75);
        assert_eq!(vals[0].timestamp, 4);
        assert_eq!(vals[1].value, 3.875);
        assert_eq!(vals[1].timestamp, 5);
        assert_eq!(vals[2].value, 7.9375);
        assert_eq!(vals[2].timestamp, 6);
        assert_eq!(vals[3].value, 15.96875);
        assert_eq!(vals[3].timestamp, 7);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[], 12, 26, 9).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_slow_plus_signal_minus_one_points_yields_empty_buffer() {
        // slow=26, signal=9 -> need slow+signal-1 = 34 points.
        // 33 points -> signal is empty -> no OsMA values.
        let points: Vec<ResampledPointD1> =
            (0..33).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 12, 26, 9).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_slow_plus_signal_minus_two_points_yields_empty_buffer() {
        // slow=5, signal=2 -> slow+signal-2 = 5 points -> empty.
        // slow+signal-1 = 6 points needed for a single value.
        let points: Vec<ResampledPointD1> =
            (0..5).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 3, 5, 2).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_slow_plus_signal_minus_one_points_yields_single_value() {
        // slow=5, signal=2 -> slow+signal-1 = 6 points -> 1 OsMA value.
        // Constant close=10 -> all EMAs = 10.0 -> MACD = 0.0 -> OsMA = 0.0.
        let points: Vec<ResampledPointD1> =
            (0..6).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 3, 5, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 6);
    }

    #[test]
    fn non_positive_fast_period_returns_error() {
        let points = vec![rp(1, 1, 10.0)];
        assert_eq!(
            compute(&points, 0, 26, 9).unwrap_err(),
            IndicatorError::InvalidOsMaFastPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3, 26, 9).unwrap_err(),
            IndicatorError::InvalidOsMaFastPeriod { n: -3 }
        );
    }

    #[test]
    fn non_positive_slow_period_returns_error() {
        let points = vec![rp(1, 1, 10.0)];
        assert_eq!(
            compute(&points, 12, 0, 9).unwrap_err(),
            IndicatorError::InvalidOsMaSlowPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, 12, -5, 9).unwrap_err(),
            IndicatorError::InvalidOsMaSlowPeriod { n: -5 }
        );
    }

    #[test]
    fn non_positive_signal_period_returns_error() {
        let points = vec![rp(1, 1, 10.0)];
        assert_eq!(
            compute(&points, 12, 26, 0).unwrap_err(),
            IndicatorError::InvalidOsMaSignalPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, 12, 26, -9).unwrap_err(),
            IndicatorError::InvalidOsMaSignalPeriod { n: -9 }
        );
    }

    #[test]
    fn timestamps_track_evaluated_bar() {
        // fast=3, slow=5, signal=2 -> start = 5+2-2 = 5.
        // 8 points -> 3 OsMA values at indices 5, 6, 7.
        let points: Vec<ResampledPointD1> =
            (0..8).map(|i| rp(i + 1, (i + 100) as i64, 10.0)).collect();
        let result = compute(&points, 3, 5, 2).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 3);
        assert_eq!(vals[0].timestamp, 105);
        assert_eq!(vals[1].timestamp, 106);
        assert_eq!(vals[2].timestamp, 107);
    }

    #[test]
    fn output_length_is_n_minus_slow_minus_signal_plus_two() {
        // 10 points, slow=5, signal=2 -> 10 - 5 - 2 + 2 = 5 OsMA values.
        let points: Vec<ResampledPointD1> =
            (0..10).map(|i| rp(i + 1, (i + 1) as i64, 10.0)).collect();
        let result = compute(&points, 3, 5, 2).unwrap();
        assert_eq!(indicator_values!(result).len(), 5);
    }

    // --- Level 2: real OsMA output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from the real EURUSD development DB.
    ///
    /// These values are **not ground-truth** — they were produced by running
    /// `osma::compute` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `fast_ema = 12`, `slow_ema = 26`, and `signal_sma = 9` (the MQL5
    /// defaults) and capturing the output. They serve as a regression
    /// baseline. The user will later compare them against an external source
    /// of truth.
    #[test]
    fn level2_real_osma_output_matches_captured_baseline() {
        let points = match test_support::fetch_eurusd_points() {
            Some(p) => p,
            None => {
                eprintln!("skipping level2_baseline_test: development DB unavailable");
                return;
            }
        };
        let result = compute(&points, 12, 26, 9).unwrap();
        let vals = indicator_values!(result);

        // 2400 points -> 2367 OsMA values (2400 - 26 - 9 + 2).
        assert_eq!(vals.len(), 2367);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1358726400);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1358726400, 0.0019515674616462273),
            (1358812800, 0.0012959000016111348),
            (1358899200, 0.0006347447741199309),
            (1358985600, 0.0006315760498687923),
            (1359072000, 0.001214602535998378),
            (1359331200, 0.0013816252842968613),
            (1359417600, 0.0016575236039562862),
            (1359504000, 0.002329526608519261),
            (1359590400, 0.0027456280578343796),
            (1359676800, 0.003169752298724725),
            (1359936000, 0.0022163355255211646),
            (1360022400, 0.0018330323939614868),
            (1360108800, 0.0008777350160591689),
            (1360195200, -0.0008027079946677507),
            (1360281600, -0.002298763380467417),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
