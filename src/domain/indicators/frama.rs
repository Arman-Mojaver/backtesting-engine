//! FrAMA: the close crossing Ehlers' fractal adaptive moving average.
//!
//! Over the last `2 * period` bars, `N1` and `N2` are the high-low ranges of
//! the recent and older halves divided by `period`, and `N3` the range of the
//! whole window divided by `2 * period`. The fractal dimension
//! `D = log2(N1 + N2) - log2(N3)` sets `alpha = exp(-4.6 * (D - 1))`, and
//! `FrAMA = FrAMA[1] + alpha * (close - FrAMA[1])`, seeded with the close at
//! bar `2 * period - 2`. Ported from `FrAMA.mq5` (`PRICE_CLOSE`). FrAMA is a
//! price overlay, so its signal is the close crossing the FrAMA line.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{CrossoverBuffers, IndicatorValue};
use crate::empty_crossover;
use crate::validate_period;

/// The long buffer is the close; the short buffer is the FrAMA line. Both
/// start at bar `2 * period - 1`, the first bar with a full window.
pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<CrossoverBuffers, IndicatorError> {
    validate_period!(period, InvalidFramaPeriod);
    let p = cast::i32_to_usize(period);
    let window_len = p.saturating_mul(2);

    let Some(seed) = points.get(window_len.saturating_sub(2)) else {
        empty_crossover!();
    };

    let mut frama = seed.close;
    let mut upcross_long_buffer = Vec::new();
    let mut upcross_short_buffer = Vec::new();
    for window in points.windows(window_len) {
        let Some(point) = window.last() else {
            continue;
        };
        let (older, recent) = window.split_at(p);
        let half = cast::usize_to_f64(p);
        let n1 = price_range(recent) / half;
        let n2 = price_range(older) / half;
        let n3 = price_range(window) / (2.0 * half);
        let alpha = (-4.6 * (fractal_dimension(n1 + n2, n3) - 1.0)).exp();
        frama = alpha.mul_add(point.close - frama, frama);
        upcross_long_buffer.push(IndicatorValue {
            timestamp: point.timestamp,
            value: point.close,
        });
        upcross_short_buffer.push(IndicatorValue {
            timestamp: point.timestamp,
            value: frama,
        });
    }

    Ok(CrossoverBuffers {
        upcross_long_buffer,
        upcross_short_buffer,
    })
}

/// MQL5 takes `log(0)` on a window with no movement in either half, which
/// poisons every later value with NaN or infinity. A move with no range inside
/// the halves is a pure trend, so it reads as a line: dimension 1, alpha 1.
fn fractal_dimension(halves_range: f64, window_range: f64) -> f64 {
    if halves_range <= 0.0 || window_range <= 0.0 {
        return 1.0;
    }
    halves_range.log2() - window_range.log2()
}

fn price_range(points: &[ResampledPointD1]) -> f64 {
    let highest = points.iter().map(|pt| pt.high).fold(f64::MIN, f64::max);
    let lowest = points.iter().map(|pt| pt.low).fold(f64::MAX, f64::min);
    highest - lowest
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;

    fn rp(id: i32, ts: i64, high: f64, low: f64, close: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: close,
            high,
            low,
            close,
            volume: 1000,
            timestamp: ts,
        }
    }

    fn points_with_closes(closes: &[f64]) -> Vec<ResampledPointD1> {
        closes
            .iter()
            .zip(1..)
            .map(|(&close, i)| rp(i, i64::from(i), close, close, close))
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
    fn constant_close_yields_frama_equal_to_close() {
        let points = points_with_closes(&[10.0; 6]);
        let result = compute(&points, 2).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![10.0; 3]);
    }

    #[test]
    fn straight_trend_has_dimension_one_and_follows_the_close() {
        // closes [1, 2, 3, 4], period 2: N1 = 1/2, N2 = 1/2, N3 = 3/4.
        // D = log2(1) - log2(3/4) = log2(4/3), so alpha = exp(-4.6 * (D - 1)).
        let points = points_with_closes(&[1.0, 2.0, 3.0, 4.0]);
        let result = compute(&points, 2).unwrap();
        let alpha = (-4.6 * ((4.0f64 / 3.0).log2() - 1.0)).exp();
        let expected = alpha.mul_add(4.0 - 3.0, 3.0);
        let frama = values(&result.upcross_short_buffer);
        assert_eq!(frama.len(), 1);
        assert!((frama[0] - expected).abs() < 1e-12);
    }

    #[test]
    fn choppy_window_has_dimension_two_and_alpha_point_zero_one() {
        // Each half spans 0..2 and so does the window: N1 = N2 = 1, N3 = 1.
        // D = log2(2) - log2(1) = 2, alpha = exp(-4.6). Seed close 1.
        let points = vec![
            rp(1, 1, 2.0, 0.0, 1.0),
            rp(2, 2, 2.0, 0.0, 1.0),
            rp(3, 3, 2.0, 0.0, 1.0),
            rp(4, 4, 2.0, 0.0, 3.0),
        ];
        let result = compute(&points, 2).unwrap();
        let expected = (-4.6f64).exp().mul_add(3.0 - 1.0, 1.0);
        let frama = values(&result.upcross_short_buffer);
        assert!((frama[0] - expected).abs() < 1e-12);
    }

    #[test]
    fn jump_between_flat_halves_reads_as_a_line() {
        // Flat halves at 1 and 2: N1 + N2 = 0 -> dimension 1, alpha 1.
        let points = points_with_closes(&[1.0, 1.0, 2.0, 2.0]);
        let result = compute(&points, 2).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![2.0]);
    }

    #[test]
    fn long_buffer_is_the_close() {
        let points = points_with_closes(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = compute(&points, 2).unwrap();
        assert_eq!(values(&result.upcross_long_buffer), vec![4.0, 5.0]);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffers() {
        let result = compute(&[], 14).unwrap();
        assert!(result.upcross_long_buffer.is_empty());
        assert!(result.upcross_short_buffer.is_empty());
    }

    #[test]
    fn fewer_than_two_period_points_yields_empty_buffers() {
        let points = points_with_closes(&[10.0; 5]);
        let result = compute(&points, 3).unwrap();
        assert!(result.upcross_long_buffer.is_empty());
        assert!(result.upcross_short_buffer.is_empty());
    }

    #[test]
    fn exactly_two_period_points_yields_single_value_at_last_bar() {
        let points = points_with_closes(&[10.0; 6]);
        let result = compute(&points, 3).unwrap();
        assert_eq!(timestamps(&result.upcross_short_buffer), vec![6]);
        assert_eq!(timestamps(&result.upcross_long_buffer), vec![6]);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = points_with_closes(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidFramaPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidFramaPeriod { n: -3 }
        );
    }

    // --- Level 2: real FrAMA output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured by running `frama::compute` on the frozen
    /// EURUSD fixture with `period = 14` (the MQL5 default). Not ground truth:
    /// a regression baseline to compare later against an external source.
    #[test]
    fn level2_real_frama_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 14).unwrap();
        let frama = &result.upcross_short_buffer;

        // 2400 points -> 2400 - 2*14 + 1 values.
        assert_eq!(frama.len(), 2373);
        assert_eq!(result.upcross_long_buffer.len(), 2373);

        assert_eq!(frama[0].timestamp, 1357862400);
        assert_eq!(frama.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(frama.last().expect("non-empty").value, 1.1268174248146727);

        let expected_first_5: &[(i64, f64)] = &[
            (1357862400, 1.3269650821524521),
            (1358121600, 1.3274587710874475),
            (1358208000, 1.3275632495410534),
            (1358294400, 1.327630095604212),
            (1358380800, 1.3280648078658408),
        ];
        for (i, (ts, val)) in expected_first_5.iter().enumerate() {
            assert_eq!(frama[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(frama[i].value, *val, "value mismatch at index {i}");
        }
    }
}
