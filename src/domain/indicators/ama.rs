//! AMA: the close crossing Kaufman's adaptive moving average.
//!
//! `ER = |open - open[period]| / sum(|open - open[1]|)` over `period` bars,
//! `SSC = ER * (fast_sc - slow_sc) + slow_sc` with `sc = 2 / (n + 1)`, and
//! `AMA = AMA[1] + SSC^2 * (open - AMA[1])`, seeded with the open at bar
//! `period - 1`. Ported from `AMA.mq5` (`PRICE_OPEN`). AMA is a price
//! overlay, not an oscillator, so its signal is the close crossing the AMA.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{CrossoverBuffers, IndicatorValue};
use crate::empty_crossover;
use crate::validate_period;

/// The long buffer is the close; the short buffer is the AMA line. Both start
/// at bar `period`, the first bar with a full efficiency-ratio window.
pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
    fast_period: i32,
    slow_period: i32,
) -> Result<CrossoverBuffers, IndicatorError> {
    validate_period!(period, InvalidAmaPeriod; fast_period, InvalidAmaFastPeriod; slow_period, InvalidAmaSlowPeriod);
    if fast_period > slow_period {
        return Err(IndicatorError::InvalidAmaPeriodOrder {
            fast: fast_period,
            slow: slow_period,
        });
    }
    let p = cast::i32_to_usize(period);
    let fast_sc = 2.0 / (f64::from(fast_period) + 1.0);
    // AMA.mq5 never assigns ExtSlowSC, which leaves it 0 and the slow period
    // unused; MetaQuotes' reference AMA sets it like the fast constant.
    let slow_sc = 2.0 / (f64::from(slow_period) + 1.0);

    let opens: Vec<f64> = points.iter().map(|pt| pt.open).collect();
    let Some(&seed) = opens.get(p.saturating_sub(1)) else {
        empty_crossover!();
    };

    let mut ama = seed;
    let mut upcross_long_buffer = Vec::with_capacity(points.len().saturating_sub(p));
    let mut upcross_short_buffer = Vec::with_capacity(points.len().saturating_sub(p));
    for (window, point) in opens
        .windows(p.saturating_add(1))
        .zip(points.iter().skip(p))
    {
        let smoothing = efficiency_ratio(window).mul_add(fast_sc - slow_sc, slow_sc);
        ama = smoothing.powi(2).mul_add(point.open - ama, ama);
        upcross_long_buffer.push(IndicatorValue {
            timestamp: point.timestamp,
            value: point.close,
        });
        upcross_short_buffer.push(IndicatorValue {
            timestamp: point.timestamp,
            value: ama,
        });
    }

    Ok(CrossoverBuffers {
        upcross_long_buffer,
        upcross_short_buffer,
    })
}

/// Net move over total path: 1 for a straight trend, near 0 for chop.
fn efficiency_ratio(window: &[f64]) -> f64 {
    let (Some(first), Some(last)) = (window.first(), window.last()) else {
        return 0.0;
    };
    let noise: f64 = window
        .iter()
        .zip(window.iter().skip(1))
        .map(|(prev, curr)| (curr - prev).abs())
        .sum();
    if noise == 0.0 {
        return 0.0;
    }
    (last - first).abs() / noise
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;

    fn rp(id: i32, ts: i64, open: f64, close: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open,
            high: open.max(close),
            low: open.min(close),
            close,
            volume: 1000,
            timestamp: ts,
        }
    }

    fn points_with_opens(opens: &[f64]) -> Vec<ResampledPointD1> {
        opens
            .iter()
            .zip(1..)
            .map(|(&open, i)| rp(i, i64::from(i), open, open))
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
    fn constant_open_yields_ama_equal_to_open() {
        let points = points_with_opens(&[10.0; 5]);
        let result = compute(&points, 2, 2, 30).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![10.0; 3]);
    }

    #[test]
    fn straight_trend_uses_fast_constant() {
        // ER = 1 on a straight line, so SSC = fast_sc = 2/3 and SSC^2 = 4/9.
        // opens [1, 2, 3], period 2, seed 2: AMA = 2 + 4/9 * (3 - 2) = 22/9.
        let points = points_with_opens(&[1.0, 2.0, 3.0]);
        let result = compute(&points, 2, 2, 30).unwrap();
        let ama = values(&result.upcross_short_buffer);
        assert_eq!(ama.len(), 1);
        assert!((ama[0] - 22.0 / 9.0).abs() < 1e-12);
    }

    #[test]
    fn choppy_market_uses_slow_constant() {
        // opens [1, 2, 1], period 2: ER = 0, so SSC = slow_sc = 2/4 and
        // SSC^2 = 1/4. Seed 2: AMA = 2 + 1/4 * (1 - 2) = 1.75.
        let points = points_with_opens(&[1.0, 2.0, 1.0]);
        let result = compute(&points, 2, 2, 3).unwrap();
        assert_eq!(values(&result.upcross_short_buffer), vec![1.75]);
    }

    #[test]
    fn partial_efficiency_blends_fast_and_slow_constants() {
        // opens [1, 3, 2], period 2: ER = 1 / 3, fast_sc = 2/3, slow_sc = 1/2.
        // SSC = 1/3 * (2/3 - 1/2) + 1/2 = 5/9. Seed 3: AMA = 3 - 25/81.
        let points = points_with_opens(&[1.0, 3.0, 2.0]);
        let result = compute(&points, 2, 2, 3).unwrap();
        let ama = values(&result.upcross_short_buffer);
        assert!((ama[0] - (3.0 - 25.0 / 81.0)).abs() < 1e-12);
    }

    #[test]
    fn long_buffer_is_the_close_while_ama_follows_the_open() {
        let points = vec![rp(1, 1, 10.0, 11.0), rp(2, 2, 10.0, 9.0)];
        let result = compute(&points, 1, 2, 30).unwrap();
        assert_eq!(values(&result.upcross_long_buffer), vec![9.0]);
        assert_eq!(values(&result.upcross_short_buffer), vec![10.0]);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffers() {
        let result = compute(&[], 10, 2, 30).unwrap();
        assert!(result.upcross_long_buffer.is_empty());
        assert!(result.upcross_short_buffer.is_empty());
    }

    #[test]
    fn exactly_period_points_yields_empty_buffers() {
        let points = points_with_opens(&[10.0; 3]);
        let result = compute(&points, 3, 2, 30).unwrap();
        assert!(result.upcross_long_buffer.is_empty());
        assert!(result.upcross_short_buffer.is_empty());
    }

    #[test]
    fn exactly_period_plus_one_points_yields_single_value_at_last_bar() {
        let points = points_with_opens(&[10.0; 4]);
        let result = compute(&points, 3, 2, 30).unwrap();
        assert_eq!(timestamps(&result.upcross_short_buffer), vec![4]);
        assert_eq!(timestamps(&result.upcross_long_buffer), vec![4]);
    }

    #[test]
    fn non_positive_periods_return_errors() {
        let points = points_with_opens(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 0, 2, 30).unwrap_err(),
            IndicatorError::InvalidAmaPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, 10, -1, 30).unwrap_err(),
            IndicatorError::InvalidAmaFastPeriod { n: -1 }
        );
        assert_eq!(
            compute(&points, 10, 2, 0).unwrap_err(),
            IndicatorError::InvalidAmaSlowPeriod { n: 0 }
        );
    }

    #[test]
    fn fast_period_above_slow_period_returns_error() {
        let points = points_with_opens(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 10, 30, 2).unwrap_err(),
            IndicatorError::InvalidAmaPeriodOrder { fast: 30, slow: 2 }
        );
    }

    #[test]
    fn equal_fast_and_slow_periods_are_accepted() {
        let points = points_with_opens(&[1.0, 2.0, 3.0]);
        assert!(compute(&points, 2, 5, 5).is_ok());
    }

    // --- Level 2: real AMA output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured by running `ama::compute` on the frozen
    /// EURUSD fixture with `period = 10, fast = 2, slow = 30` (the MQL5
    /// defaults). Not ground truth: a regression baseline to compare later
    /// against an external source.
    #[test]
    fn level2_real_ama_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 10, 2, 30).unwrap();
        let ama = &result.upcross_short_buffer;

        // 2400 points -> 2400 - 10 values.
        assert_eq!(ama.len(), 2390);
        assert_eq!(result.upcross_long_buffer.len(), 2390);

        assert_eq!(ama[0].timestamp, 1355788800);
        assert_eq!(ama.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(ama.last().expect("non-empty").value, 1.1201489383987886);

        let expected_first_5: &[(i64, f64)] = &[
            (1355788800, 1.318137886753677),
            (1355875200, 1.3183133295503764),
            (1355961600, 1.3183768630175514),
            (1356048000, 1.3194358212853181),
            (1356307200, 1.3191957195714366),
        ];
        for (i, (ts, val)) in expected_first_5.iter().enumerate() {
            assert_eq!(ama[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(ama[i].value, *val, "value mismatch at index {i}");
        }
    }
}
