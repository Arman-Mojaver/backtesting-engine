//! Williams' Percent Range (WPR): how far the close sits below the recent
//! highest high, as a share of the recent high-low range.
//!
//! `%R = -100 * (highest - close) / (highest - lowest)` with the highest high
//! and lowest low over the last `period` bars. Ported from `WPR.mq5`. Values
//! run from -100 (close at the lowest low) to 0 (close at the highest high),
//! so its overbought/oversold levels are negative, e.g. -20 and -80.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, ThresholdBuffer};
use crate::validate_period;

pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
    high_threshold: i32,
    low_threshold: i32,
) -> Result<ThresholdBuffer, IndicatorError> {
    validate_period!(period, InvalidWprPeriod);

    for threshold in [high_threshold, low_threshold] {
        if !(-100..=0).contains(&threshold) {
            return Err(IndicatorError::InvalidWprThresholdRange { value: threshold });
        }
    }

    if low_threshold >= high_threshold {
        return Err(IndicatorError::InvalidWprThresholdOrder {
            high: high_threshold,
            low: low_threshold,
        });
    }

    // MQL5 carries the previous value through a window with no range; the
    // buffer before the first value holds 0.
    let mut previous: f64 = 0.0;
    let mut items = Vec::new();
    for window in points.windows(cast::i32_to_usize(period)) {
        let Some(last) = window.last() else {
            continue;
        };
        let value = percent_range(window).unwrap_or(previous);
        previous = value;
        items.push(IndicatorValue {
            timestamp: last.timestamp,
            value,
        });
    }

    Ok(ThresholdBuffer {
        items,
        high_threshold: f64::from(high_threshold),
        low_threshold: f64::from(low_threshold),
    })
}

/// `None` when the window has no high-low range.
fn percent_range(window: &[ResampledPointD1]) -> Option<f64> {
    let mut lowest = f64::INFINITY;
    let mut highest = f64::NEG_INFINITY;
    let mut close: f64 = 0.0;
    for point in window {
        lowest = lowest.min(point.low);
        highest = highest.max(point.high);
        close = point.close;
    }

    if highest == lowest {
        return None;
    }
    Some(-(highest - close) * 100.0 / (highest - lowest))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::indicator_values;

    fn rp(id: i32, ts: i64, high: f64, low: f64, close: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: close,
            high,
            low,
            close,
            volume: 100,
            timestamp: ts,
        }
    }

    /// High, low and close all equal the price, so each bar has no range of its own.
    fn points_with_prices(prices: &[f64]) -> Vec<ResampledPointD1> {
        prices
            .iter()
            .zip(1..)
            .map(|(&price, i)| rp(i, i64::from(i), price, price, price))
            .collect()
    }

    fn values(buffer: &ThresholdBuffer) -> Vec<f64> {
        indicator_values!(buffer).iter().map(|v| v.value).collect()
    }

    fn timestamps(buffer: &ThresholdBuffer) -> Vec<i64> {
        indicator_values!(buffer)
            .iter()
            .map(|v| v.timestamp)
            .collect()
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn close_at_highest_high_yields_zero() {
        let points = points_with_prices(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = compute(&points, 3, -20, -80).unwrap();
        assert_eq!(values(&result), vec![0.0, 0.0, 0.0]);
        assert_eq!(timestamps(&result), vec![3, 4, 5]);
    }

    #[test]
    fn close_at_lowest_low_yields_minus_100() {
        let points = points_with_prices(&[5.0, 4.0, 3.0, 2.0, 1.0]);
        let result = compute(&points, 3, -20, -80).unwrap();
        assert_eq!(values(&result), vec![-100.0, -100.0, -100.0]);
    }

    #[test]
    fn range_uses_high_and_low_not_close() {
        // Highest high 4, lowest low 0, close 1 -> -(4 - 1) * 100 / 4 = -75.
        let points = vec![rp(1, 1, 4.0, 2.0, 3.0), rp(2, 2, 2.0, 0.0, 1.0)];
        let result = compute(&points, 2, -20, -80).unwrap();
        assert_eq!(values(&result), vec![-75.0]);
    }

    #[test]
    fn flat_window_carries_previous_value_like_mql5() {
        // Window [1, 2]: close 2 at the high -> 0. Window [2, 0] -> -100.
        // Windows [0, 0] and [0, 0] have no range and repeat -100.
        let points = points_with_prices(&[1.0, 2.0, 0.0, 0.0, 0.0]);
        let result = compute(&points, 2, -20, -80).unwrap();
        assert_eq!(values(&result), vec![0.0, -100.0, -100.0, -100.0]);
    }

    #[test]
    fn flat_first_window_yields_zero_like_mql5() {
        let points = points_with_prices(&[2.0; 3]);
        let result = compute(&points, 2, -20, -80).unwrap();
        assert_eq!(values(&result), vec![0.0, 0.0]);
    }

    #[test]
    fn thresholds_carried_through_as_levels() {
        let points = points_with_prices(&[1.0, 2.0, 3.0]);
        let result = compute(&points, 2, -20, -80).unwrap();
        assert_eq!(result.high_threshold, -20.0);
        assert_eq!(result.low_threshold, -80.0);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_items() {
        let result = compute(&[], 14, -20, -80).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_period_points_yields_empty_items() {
        let points = points_with_prices(&[1.0, 2.0]);
        let result = compute(&points, 3, -20, -80).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_points_yields_single_value_at_last_bar() {
        let points = points_with_prices(&[1.0, 2.0, 3.0]);
        let result = compute(&points, 3, -20, -80).unwrap();
        assert_eq!(timestamps(&result), vec![3]);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = points_with_prices(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 0, -20, -80).unwrap_err(),
            IndicatorError::InvalidWprPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3, -20, -80).unwrap_err(),
            IndicatorError::InvalidWprPeriod { n: -3 }
        );
    }

    #[test]
    fn threshold_outside_minus_100_0_returns_error() {
        let points = points_with_prices(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 2, 20, -80).unwrap_err(),
            IndicatorError::InvalidWprThresholdRange { value: 20 }
        );
        assert_eq!(
            compute(&points, 2, -20, -101).unwrap_err(),
            IndicatorError::InvalidWprThresholdRange { value: -101 }
        );
    }

    #[test]
    fn low_threshold_not_below_high_returns_error() {
        let points = points_with_prices(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 2, -80, -20).unwrap_err(),
            IndicatorError::InvalidWprThresholdOrder {
                high: -80,
                low: -20
            }
        );
        assert_eq!(
            compute(&points, 2, -50, -50).unwrap_err(),
            IndicatorError::InvalidWprThresholdOrder {
                high: -50,
                low: -50
            }
        );
    }

    // --- Level 2: real WPR output on real EURUSD data (placeholder baseline) ---

    /// Baseline captured on the frozen EURUSD fixture with `period = 14` (the
    /// MQL5 default), and matched bit for bit by a direct Python
    /// transcription of the `WPR.mq5` loop. Not checked against MetaTrader
    /// output itself.
    #[test]
    fn level2_real_wpr_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 14, -20, -80).unwrap();
        let wpr = indicator_values!(result);

        // 2400 points -> 2400 - 14 + 1 values.
        assert_eq!(wpr.len(), 2387);

        assert_eq!(wpr[0].timestamp, 1356048000);
        assert_eq!(wpr.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(wpr.last().expect("non-empty").value, -91.12286890064642);

        let expected_first_5: &[(i64, f64)] = &[
            (1356048000, -28.070582772231138),
            (1356307200, -28.0009287206872),
            (1356480000, -20.269328999303266),
            (1356566400, -17.042024611098007),
            (1356652800, -22.032693674484356),
        ];
        for (i, (ts, val)) in expected_first_5.iter().enumerate() {
            assert_eq!(wpr[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(wpr[i].value, *val, "value mismatch at index {i}");
        }
    }
}
