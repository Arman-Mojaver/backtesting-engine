//! Stochastic Oscillator: where the close sits within the recent high-low range.
//!
//! Each bar's range spans the lowest low and highest high of the last
//! `k_period` bars. `%K = 100 * sum(close - lowest) / sum(highest - lowest)`
//! with both sums over the last `slowing` ranges. Ported from `Stochastic.mq5`.
//! The %D signal line is not computed: the engine trades %K leaving its
//! high/low zones, so %D would feed no signal.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, ThresholdBuffer};
use crate::validate_period;

#[derive(Debug, Clone, Copy)]
struct PriceRange {
    timestamp: i64,
    close_above_lowest: f64,
    highest_above_lowest: f64,
}

pub fn compute(
    points: &[ResampledPointD1],
    k_period: i32,
    slowing: i32,
    high_threshold: i32,
    low_threshold: i32,
) -> Result<ThresholdBuffer, IndicatorError> {
    validate_period!(k_period, InvalidStochasticKPeriod; slowing, InvalidStochasticSlowing);

    for threshold in [high_threshold, low_threshold] {
        if !(0..=100).contains(&threshold) {
            return Err(IndicatorError::InvalidStochasticThresholdRange { value: threshold });
        }
    }

    if low_threshold >= high_threshold {
        return Err(IndicatorError::InvalidStochasticThresholdOrder {
            high: high_threshold,
            low: low_threshold,
        });
    }

    let ranges: Vec<PriceRange> = points
        .windows(cast::i32_to_usize(k_period))
        .map(price_range)
        .collect();

    Ok(ThresholdBuffer {
        items: ranges
            .windows(cast::i32_to_usize(slowing))
            .map(compute_stochastic_point)
            .collect(),
        high_threshold: f64::from(high_threshold),
        low_threshold: f64::from(low_threshold),
    })
}

fn price_range(window: &[ResampledPointD1]) -> PriceRange {
    let mut lowest = f64::INFINITY;
    let mut highest = f64::NEG_INFINITY;
    let mut close: f64 = 0.0;
    let mut timestamp: i64 = 0;
    for point in window {
        lowest = lowest.min(point.low);
        highest = highest.max(point.high);
        close = point.close;
        timestamp = point.timestamp;
    }

    PriceRange {
        timestamp,
        close_above_lowest: close - lowest,
        highest_above_lowest: highest - lowest,
    }
}

fn compute_stochastic_point(window: &[PriceRange]) -> IndicatorValue {
    let mut close_above_lowest: f64 = 0.0;
    let mut highest_above_lowest: f64 = 0.0;
    let mut timestamp: i64 = 0;
    for range in window {
        close_above_lowest += range.close_above_lowest;
        highest_above_lowest += range.highest_above_lowest;
        timestamp = range.timestamp;
    }

    // MQL5 reports 100 when the window has no range, a flat market included.
    let value = if highest_above_lowest == 0.0 {
        100.0
    } else {
        close_above_lowest / highest_above_lowest * 100.0
    };

    IndicatorValue { timestamp, value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::ResampledPointD1;
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
    fn close_at_highest_high_yields_max_stochastic() {
        let points = points_with_prices(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = compute(&points, 3, 1, 80, 20).unwrap();
        assert_eq!(values(&result), vec![100.0, 100.0, 100.0]);
        assert_eq!(timestamps(&result), vec![3, 4, 5]);
    }

    #[test]
    fn close_at_lowest_low_yields_min_stochastic() {
        let points = points_with_prices(&[5.0, 4.0, 3.0, 2.0, 1.0]);
        let result = compute(&points, 3, 1, 80, 20).unwrap();
        assert_eq!(values(&result), vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn flat_market_yields_max_stochastic_like_mql5() {
        let points = points_with_prices(&[2.0; 4]);
        let result = compute(&points, 2, 2, 80, 20).unwrap();
        assert_eq!(values(&result), vec![100.0, 100.0]);
    }

    #[test]
    fn range_uses_high_and_low_not_close() {
        // Range over both bars: lowest low 0, highest high 4; close 1 -> 25.
        let points = vec![rp(1, 1, 4.0, 2.0, 3.0), rp(2, 2, 2.0, 0.0, 1.0)];
        let result = compute(&points, 2, 1, 80, 20).unwrap();
        assert_eq!(values(&result), vec![25.0]);
    }

    #[test]
    fn slowing_sums_ranges_before_dividing() {
        // k_period 1, so each bar's range is its own high-low.
        // Bar 1: close - low = 1, high - low = 4. Bar 2: 3 and 4. Bar 3: 0 and 2.
        // Slowing 2: (1 + 3) / (4 + 4) = 50, then (3 + 0) / (4 + 2) = 50.
        // Averaging per-bar ratios instead would give 50 and 37.5.
        let points = vec![
            rp(1, 1, 4.0, 0.0, 1.0),
            rp(2, 2, 4.0, 0.0, 3.0),
            rp(3, 3, 2.0, 0.0, 0.0),
        ];
        let result = compute(&points, 1, 2, 80, 20).unwrap();
        assert_eq!(values(&result), vec![50.0, 50.0]);
        assert_eq!(timestamps(&result), vec![2, 3]);
    }

    #[test]
    fn thresholds_carried_through_as_levels() {
        let points = points_with_prices(&[1.0, 2.0, 3.0]);
        let result = compute(&points, 2, 1, 80, 20).unwrap();
        assert_eq!(result.high_threshold, 80.0);
        assert_eq!(result.low_threshold, 20.0);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_items() {
        let result = compute(&[], 5, 3, 80, 20).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn one_point_short_of_warm_up_yields_empty_items() {
        // k_period 3 + slowing 2 needs 4 points.
        let points = points_with_prices(&[1.0, 2.0, 3.0]);
        let result = compute(&points, 3, 2, 80, 20).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_warm_up_points_yields_single_value_at_last_bar() {
        let points = points_with_prices(&[1.0, 2.0, 3.0, 4.0]);
        let result = compute(&points, 3, 2, 80, 20).unwrap();
        assert_eq!(timestamps(&result), vec![4]);
    }

    #[test]
    fn non_positive_k_period_returns_error() {
        let points = points_with_prices(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 0, 3, 80, 20).unwrap_err(),
            IndicatorError::InvalidStochasticKPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3, 3, 80, 20).unwrap_err(),
            IndicatorError::InvalidStochasticKPeriod { n: -3 }
        );
    }

    #[test]
    fn non_positive_slowing_returns_error() {
        let points = points_with_prices(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 5, 0, 80, 20).unwrap_err(),
            IndicatorError::InvalidStochasticSlowing { n: 0 }
        );
    }

    #[test]
    fn threshold_outside_0_100_returns_error() {
        let points = points_with_prices(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 2, 1, 101, 20).unwrap_err(),
            IndicatorError::InvalidStochasticThresholdRange { value: 101 }
        );
        assert_eq!(
            compute(&points, 2, 1, 80, 101).unwrap_err(),
            IndicatorError::InvalidStochasticThresholdRange { value: 101 }
        );
    }

    #[test]
    fn low_threshold_not_below_high_returns_error() {
        let points = points_with_prices(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 2, 1, 20, 80).unwrap_err(),
            IndicatorError::InvalidStochasticThresholdOrder { high: 20, low: 80 }
        );
        assert_eq!(
            compute(&points, 2, 1, 50, 50).unwrap_err(),
            IndicatorError::InvalidStochasticThresholdOrder { high: 50, low: 50 }
        );
    }

    // --- Level 2: real stochastic output on real EURUSD data (placeholder baseline) ---

    /// Baseline captured on the frozen EURUSD fixture with `k_period = 5`,
    /// `slowing = 3` (the MQL5 defaults), and matched bit for bit by a direct
    /// Python transcription of the `Stochastic.mq5` loops. Not checked against
    /// MetaTrader output itself.
    #[test]
    fn level2_real_stochastic_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 5, 3, 80, 20).unwrap();
        let stochastic = indicator_values!(result);

        // 2400 points -> 2400 - (5 - 1) - (3 - 1) values.
        assert_eq!(stochastic.len(), 2394);

        assert_eq!(stochastic[0].timestamp, 1355270400);
        assert_eq!(stochastic.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(stochastic.last().expect("non-empty").value, 16.301161017943);

        let expected_first_5: &[(i64, f64)] = &[
            (1355270400, 54.043860874423935),
            (1355356800, 75.52143684820406),
            (1355443200, 91.49753154141528),
            (1355702400, 91.8239812524413),
            (1355788800, 93.44928272184865),
        ];
        for (i, (ts, val)) in expected_first_5.iter().enumerate() {
            assert_eq!(
                stochastic[i].timestamp, *ts,
                "timestamp mismatch at index {i}"
            );
            assert_eq!(stochastic[i].value, *val, "value mismatch at index {i}");
        }
    }
}
