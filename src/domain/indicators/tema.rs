//! TEMA: the close crossing its triple exponential moving average.
//!
//! `TEMA = 3 * EMA - 3 * EMA(EMA) + EMA(EMA(EMA))`, every EMA over `period`.
//! Ported from `TEMA.mq5` (`PRICE_CLOSE`). TEMA is a price overlay, not an
//! oscillator, so its signal is the close crossing the TEMA line.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::indicators::moving_averages::ema;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{CrossoverBuffers, IndicatorValue};
use crate::empty_crossover;
use crate::validate_period;

/// The long buffer is the close; the short buffer is the TEMA line. Both
/// start at the first bar where the EMA of the EMA of the EMA exists
/// (`3 * period - 3`).
pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<CrossoverBuffers, IndicatorError> {
    validate_period!(period, InvalidTemaPeriod);
    let p = cast::i32_to_usize(period);

    let closes: Vec<f64> = points.iter().map(|pt| pt.close).collect();
    let ema1 = ema(&closes, p);
    let ema_of_ema = ema(&ema1, p);
    let ema_of_ema_of_ema = ema(&ema_of_ema, p);

    if ema_of_ema_of_ema.is_empty() {
        empty_crossover!();
    }

    let lag = p.saturating_sub(1);
    let start = lag.saturating_mul(3);
    let (upcross_long_buffer, upcross_short_buffer) = ema1
        .iter()
        .skip(lag.saturating_mul(2))
        .zip(ema_of_ema.iter().skip(lag))
        .zip(ema_of_ema_of_ema.iter())
        .zip(points.iter().skip(start))
        .map(|(((ema, ema_of_ema), ema_of_ema_of_ema), point)| {
            let close = IndicatorValue {
                timestamp: point.timestamp,
                value: point.close,
            };
            let tema = IndicatorValue {
                timestamp: point.timestamp,
                value: 3.0f64.mul_add(ema - ema_of_ema, *ema_of_ema_of_ema),
            };
            (close, tema)
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
    fn constant_close_yields_tema_equal_to_close() {
        // Every EMA = 10 -> TEMA = 3*10 - 3*10 + 10 = 10.
        let points = points_with_closes(&[10.0; 6]);
        let result = compute(&points, 2).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![10.0; 3]);
        assert_eq!(values(&result.upcross_long_buffer), vec![10.0; 3]);
    }

    #[test]
    fn linearly_increasing_close_yields_hand_computed_tema() {
        // close = [1..6], period 2: each EMA lags its input by 0.5, so
        // EMA = c - 0.5, EMA(EMA) = c - 1, EMA(EMA(EMA)) = c - 1.5 and
        // TEMA = 3(c - 0.5) - 3(c - 1) + (c - 1.5) = c: TEMA removes the
        // EMA lag on a linear trend, so it equals the close.
        let points = points_with_closes(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        let result = compute(&points, 2).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![4.0, 5.0, 6.0]);
        assert_eq!(values(&result.upcross_long_buffer), vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn price_jump_yields_hand_computed_tema() {
        // close = [1, 1, 1, 1, 4], period 2, k = 2/3.
        // EMA1 = [1, 1, 1, 3]; EMA2 = [1, 1, 7/3]; EMA3 = [1, 17/9].
        // TEMA = [3*1 - 3*1 + 1, 3*3 - 3*7/3 + 17/9] = [1, 35/9].
        let points = points_with_closes(&[1.0, 1.0, 1.0, 1.0, 4.0]);
        let result = compute(&points, 2).unwrap();
        let tema = values(&result.upcross_short_buffer);
        assert_eq!(tema.len(), 2);
        assert!((tema[0] - 1.0).abs() < 1e-12);
        assert!((tema[1] - 35.0 / 9.0).abs() < 1e-12);
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
    fn fewer_than_three_period_minus_two_points_yields_empty_buffers() {
        // period 3 needs 3*3 - 2 = 7 points for one value.
        let points = points_with_closes(&[10.0; 6]);
        let result = compute(&points, 3).unwrap();
        assert!(result.upcross_long_buffer.is_empty());
        assert!(result.upcross_short_buffer.is_empty());
    }

    #[test]
    fn exactly_three_period_minus_two_points_yields_single_value() {
        let points = points_with_closes(&[10.0; 7]);
        let result = compute(&points, 3).unwrap();
        assert_eq!(timestamps(&result.upcross_short_buffer), vec![7]);
        assert_eq!(timestamps(&result.upcross_long_buffer), vec![7]);
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
            IndicatorError::InvalidTemaPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidTemaPeriod { n: -3 }
        );
    }

    #[test]
    fn buffers_share_the_timestamps_of_the_evaluated_bars() {
        // period 2, 6 points -> 3 values at indices 3..=5.
        let points: Vec<ResampledPointD1> = (0..6)
            .map(|i| rp(i + 1, i64::from(i + 100), 10.0))
            .collect();
        let result = compute(&points, 2).unwrap();
        assert_eq!(
            timestamps(&result.upcross_short_buffer),
            vec![103, 104, 105]
        );
        assert_eq!(
            timestamps(&result.upcross_long_buffer),
            timestamps(&result.upcross_short_buffer)
        );
    }

    // --- Level 2: real TEMA output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured by running `tema::compute` on the frozen
    /// EURUSD fixture with `period = 14` (the MQL5 default). Not ground truth:
    /// a regression baseline to compare later against an external source.
    #[test]
    fn level2_real_tema_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 14).unwrap();
        let tema = &result.upcross_short_buffer;

        // 2400 points -> 2400 - 3*14 + 3 values.
        assert_eq!(tema.len(), 2361);
        assert_eq!(result.upcross_long_buffer.len(), 2361);

        assert_eq!(tema[0].timestamp, 1359417600);
        assert_eq!(tema.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(tema.last().expect("non-empty").value, 1.1027077579685758);

        let expected_first_5: &[(i64, f64)] = &[
            (1359417600, 1.3457530418403334),
            (1359504000, 1.3509040690933598),
            (1359590400, 1.3552558176710447),
            (1359676800, 1.3601290629723344),
            (1359936000, 1.3587812663680143),
        ];
        for (i, (ts, val)) in expected_first_5.iter().enumerate() {
            assert_eq!(tema[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(tema[i].value, *val, "value mismatch at index {i}");
        }
    }
}
