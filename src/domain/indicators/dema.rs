//! DEMA: the close crossing its double exponential moving average.
//!
//! `DEMA = 2 * EMA(close, period) - EMA(EMA(close, period), period)`.
//! Ported from `DEMA.mq5` (`PRICE_CLOSE`). DEMA is a price overlay, not an
//! oscillator, so its signal is the close crossing the DEMA line.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::indicators::moving_averages::ema;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{CrossoverBuffers, IndicatorValue};
use crate::empty_crossover;
use crate::validate_period;

/// The long buffer is the close; the short buffer is the DEMA line. Both
/// start at the first bar where the EMA of the EMA exists (`2 * period - 2`).
pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<CrossoverBuffers, IndicatorError> {
    validate_period!(period, InvalidDemaPeriod);
    let p = cast::i32_to_usize(period);

    let closes: Vec<f64> = points.iter().map(|pt| pt.close).collect();
    let ema1 = ema(&closes, p);
    let ema_of_ema = ema(&ema1, p);

    if ema_of_ema.is_empty() {
        empty_crossover!();
    }

    let start = p.saturating_mul(2).saturating_sub(2);
    let (upcross_long_buffer, upcross_short_buffer) = ema1
        .iter()
        .skip(p.saturating_sub(1))
        .zip(ema_of_ema.iter())
        .zip(points.iter().skip(start))
        .map(|((ema, ema_of_ema), point)| {
            let close = IndicatorValue {
                timestamp: point.timestamp,
                value: point.close,
            };
            let dema = IndicatorValue {
                timestamp: point.timestamp,
                value: 2.0f64.mul_add(*ema, -ema_of_ema),
            };
            (close, dema)
        })
        .unzip();

    Ok(CrossoverBuffers {
        upcross_long_buffer,
        upcross_short_buffer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::ResampledPointD1;

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

    fn points_with_closes(closes: &[f64]) -> Vec<ResampledPointD1> {
        closes
            .iter()
            .zip(1..)
            .map(|(&close, i)| rp(i, i64::from(i), close))
            .collect()
    }

    fn values(buffer: &[IndicatorValue]) -> Vec<f64> {
        buffer.iter().map(|v| v.value).collect()
    }

    fn timestamps(buffer: &[IndicatorValue]) -> Vec<i64> {
        buffer.iter().map(|v| v.timestamp).collect()
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_close_yields_dema_equal_to_close() {
        // EMA = EMA(EMA) = 10 -> DEMA = 2*10 - 10 = 10.
        let points = points_with_closes(&[10.0; 6]);
        let result = compute(&points, 2).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![10.0; 4]);
        assert_eq!(values(&result.upcross_long_buffer), vec![10.0; 4]);
    }

    #[test]
    fn linearly_increasing_close_yields_hand_computed_dema() {
        // close = [1..6], period 2.
        // EMA1 = [1.5, 2.5, 3.5, 4.5, 5.5]; EMA2 = [2.0, 3.0, 4.0, 5.0].
        // DEMA = 2*EMA1[k+1] - EMA2[k] = [3.0, 4.0, 5.0, 6.0]: DEMA removes
        // the EMA lag on a linear trend, so it equals the close.
        let points = points_with_closes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let result = compute(&points, 2).unwrap();
        assert_eq!(
            values(&result.upcross_short_buffer),
            vec![3.0, 4.0, 5.0, 6.0]
        );
        assert_eq!(
            values(&result.upcross_long_buffer),
            vec![3.0, 4.0, 5.0, 6.0]
        );
    }

    #[test]
    fn price_jump_yields_hand_computed_dema() {
        // close = [1, 1, 1, 4], period 2, k = 2/3.
        // EMA1 = [1, 1, 3]; EMA2 = [1, 7/3].
        // DEMA = [2*1 - 1, 2*3 - 7/3] = [1, 11/3].
        let points = points_with_closes(&[1.0, 1.0, 1.0, 4.0]);
        let result = compute(&points, 2).unwrap();
        let dema = values(&result.upcross_short_buffer);
        assert_eq!(dema.len(), 2);
        assert!((dema[0] - 1.0).abs() < 1e-12);
        assert!((dema[1] - 11.0 / 3.0).abs() < 1e-12);
        assert_eq!(values(&result.upcross_long_buffer), vec![1.0, 4.0]);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffers() {
        let result = compute(&[], 14).unwrap();
        assert!(result.upcross_long_buffer.is_empty());
        assert!(result.upcross_short_buffer.is_empty());
    }

    #[test]
    fn fewer_than_two_period_minus_one_points_yields_empty_buffers() {
        // period 3 needs 2*3 - 1 = 5 points for one value.
        let points = points_with_closes(&[10.0; 4]);
        let result = compute(&points, 3).unwrap();
        assert!(result.upcross_long_buffer.is_empty());
        assert!(result.upcross_short_buffer.is_empty());
    }

    #[test]
    fn exactly_two_period_minus_one_points_yields_single_value() {
        let points = points_with_closes(&[10.0; 5]);
        let result = compute(&points, 3).unwrap();
        assert_eq!(timestamps(&result.upcross_short_buffer), vec![5]);
        assert_eq!(timestamps(&result.upcross_long_buffer), vec![5]);
    }

    #[test]
    fn period_one_yields_close_for_every_bar() {
        let points = points_with_closes(&[1.0, 3.0, 2.0]);
        let result = compute(&points, 1).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![1.0, 3.0, 2.0]);
        assert_eq!(timestamps(&result.upcross_short_buffer), vec![1, 2, 3]);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = vec![rp(1, 1, 10.0)];
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidDemaPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidDemaPeriod { n: -3 }
        );
    }

    #[test]
    fn buffers_share_the_timestamps_of_the_evaluated_bars() {
        // period 2, 6 points -> 4 values at indices 2..=5.
        let points: Vec<ResampledPointD1> = (0..6)
            .map(|i| rp(i + 1, i64::from(i + 100), 10.0))
            .collect();
        let result = compute(&points, 2).unwrap();
        assert_eq!(
            timestamps(&result.upcross_short_buffer),
            vec![102, 103, 104, 105]
        );
        assert_eq!(
            timestamps(&result.upcross_long_buffer),
            timestamps(&result.upcross_short_buffer)
        );
    }

    // --- Level 2: real DEMA output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured by running `dema::compute` on the frozen
    /// EURUSD fixture with `period = 14` (the MQL5 default). Not ground truth:
    /// a regression baseline to compare later against an external source.
    #[test]
    fn level2_real_dema_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 14).unwrap();
        let dema = &result.upcross_short_buffer;

        // 2400 points -> 2400 - 2*14 + 2 values.
        assert_eq!(dema.len(), 2374);
        assert_eq!(result.upcross_long_buffer.len(), 2374);

        assert_eq!(dema[0].timestamp, 1357776000);
        assert_eq!(dema.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(dema.last().expect("non-empty").value, 1.1091407461319778);

        let expected_first_5: &[(i64, f64)] = &[
            (1357776000, 1.3140861473454757),
            (1357862400, 1.3191179141556113),
            (1358121600, 1.3240333671969025),
            (1358208000, 1.3259938031755272),
            (1358294400, 1.327355840809685),
        ];
        for (i, (ts, val)) in expected_first_5.iter().enumerate() {
            assert_eq!(dema[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(dema[i].value, *val, "value mismatch at index {i}");
        }
    }
}
