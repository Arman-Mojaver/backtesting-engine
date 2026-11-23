//! Commodity Channel Index (CCI): how far the typical price strays from its
//! mean, in units of mean absolute deviation.
//!
//! `TP = (high + low + close) / 3`, `SMA = SMA(TP, period)` and
//! `CCI = (TP - SMA) / (0.015 * mean(|TP - SMA|))` over the last `period`
//! typical prices. Ported from `CCI.mq5` (`PRICE_TYPICAL`).
use crate::domain::cast;
use crate::domain::indicators::moving_averages::sma;
use crate::domain::indicators::{IndicatorError, typical_price};
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorBuffer};
use crate::validate_period;

pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<OscillatorBuffer, IndicatorError> {
    validate_period!(period, InvalidCciPeriod);
    let p = cast::i32_to_usize(period);
    // MQL5 folds Lambert's 0.015 constant and the 1/period of the mean into one factor.
    let deviation_scale = 0.015 / cast::usize_to_f64(p);

    let typical_prices: Vec<f64> = points.iter().map(typical_price).collect();
    let means = sma(&typical_prices, p);

    let items = typical_prices
        .windows(p)
        .zip(means)
        .zip(points.iter().skip(p.saturating_sub(1)))
        .map(|((window, mean), point)| {
            let scaled_mean_deviation =
                window.iter().map(|tp| (tp - mean).abs()).sum::<f64>() * deviation_scale;
            // MQL5 reports 0 for a flat window instead of dividing by zero.
            let value = if scaled_mean_deviation == 0.0 {
                0.0
            } else {
                (typical_price(point) - mean) / scaled_mean_deviation
            };
            IndicatorValue {
                timestamp: point.timestamp,
                value,
            }
        })
        .collect();

    Ok(OscillatorBuffer { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::indicator_values;

    /// High, low and close all equal `price`, so the typical price is `price`.
    fn rp(id: i32, ts: i64, price: f64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: price,
            high: price,
            low: price,
            close: price,
            volume: 1000,
            timestamp: ts,
        }
    }

    fn points_with_prices(prices: &[f64]) -> Vec<ResampledPointD1> {
        prices
            .iter()
            .zip(1..)
            .map(|(&price, i)| rp(i, i64::from(i), price))
            .collect()
    }

    fn values(buffer: &OscillatorBuffer) -> Vec<f64> {
        indicator_values!(buffer).iter().map(|v| v.value).collect()
    }

    fn timestamps(buffer: &OscillatorBuffer) -> Vec<i64> {
        indicator_values!(buffer)
            .iter()
            .map(|v| v.timestamp)
            .collect()
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn flat_typical_price_yields_zero_like_mql5() {
        let points = points_with_prices(&[2.0; 4]);
        let result = compute(&points, 3).unwrap();
        assert_eq!(values(&result), vec![0.0, 0.0]);
        assert_eq!(timestamps(&result), vec![3, 4]);
    }

    #[test]
    fn rising_typical_price_yields_hand_computed_cci() {
        // Window [1, 2, 3]: SMA = 2, mean |TP - SMA| = 2/3,
        // CCI = (3 - 2) / (0.015 * 2/3) = 100.
        let points = points_with_prices(&[1.0, 2.0, 3.0, 4.0]);
        let result = compute(&points, 3).unwrap();
        let cci = values(&result);
        assert_eq!(cci.len(), 2);
        for value in cci {
            assert!((value - 100.0).abs() < 1e-9);
        }
    }

    #[test]
    fn falling_typical_price_yields_negative_cci() {
        let points = points_with_prices(&[3.0, 2.0, 1.0]);
        let result = compute(&points, 3).unwrap();
        let cci = values(&result);
        assert_eq!(cci.len(), 1);
        assert!((cci[0] + 100.0).abs() < 1e-9);
    }

    #[test]
    fn spike_yields_hand_computed_cci() {
        // Window [1, 1, 4]: SMA = 2, mean |TP - SMA| = (1 + 1 + 2) / 3 = 4/3,
        // CCI = (4 - 2) / (0.015 * 4/3) = 100.
        // Window [1, 4, 1]: SMA = 2, same deviation, CCI = (1 - 2) / 0.02 = -50.
        let points = points_with_prices(&[1.0, 1.0, 4.0, 1.0]);
        let result = compute(&points, 3).unwrap();
        let cci = values(&result);
        assert!((cci[0] - 100.0).abs() < 1e-9);
        assert!((cci[1] + 50.0).abs() < 1e-9);
    }

    #[test]
    fn typical_price_averages_high_low_and_close() {
        // TP = (3 + 0 + 0) / 3 = 1 equals the flat neighbours, so the window
        // is flat although the high spiked.
        let mut spiked = rp(2, 2, 0.0);
        spiked.high = 3.0;
        let points = vec![rp(1, 1, 1.0), spiked, rp(3, 3, 1.0)];
        let result = compute(&points, 3).unwrap();
        assert_eq!(values(&result), vec![0.0]);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_items() {
        let result = compute(&[], 14).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_period_points_yields_empty_items() {
        let points = points_with_prices(&[1.0, 2.0]);
        let result = compute(&points, 3).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_points_yields_single_value_at_last_bar() {
        let points = points_with_prices(&[1.0, 2.0, 3.0]);
        let result = compute(&points, 3).unwrap();
        assert_eq!(timestamps(&result), vec![3]);
    }

    #[test]
    fn period_one_yields_zero_everywhere() {
        let points = points_with_prices(&[1.0, 5.0, 2.0]);
        let result = compute(&points, 1).unwrap();
        assert_eq!(values(&result), vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = points_with_prices(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidCciPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidCciPeriod { n: -3 }
        );
    }

    // --- Level 2: real CCI output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured by running `cci::compute` on the frozen
    /// EURUSD fixture with `period = 14` (the MQL5 default). Not ground truth:
    /// a regression baseline to compare later against an external source.
    #[test]
    fn level2_real_cci_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 14).unwrap();
        let cci = indicator_values!(result);

        // 2400 points -> 2400 - 14 + 1 values.
        assert_eq!(cci.len(), 2387);

        assert_eq!(cci[0].timestamp, 1356048000);
        assert_eq!(cci.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(cci.last().expect("non-empty").value, -175.47182069189364);

        let expected_first_5: &[(i64, f64)] = &[
            (1356048000, 78.20416353147829),
            (1356307200, 67.0265856848857),
            (1356480000, 71.33408194872946),
            (1356566400, 76.55400677798585),
            (1356652800, 51.68371405606534),
        ];
        for (i, (ts, val)) in expected_first_5.iter().enumerate() {
            assert_eq!(cci[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(cci[i].value, *val, "value mismatch at index {i}");
        }
    }
}
