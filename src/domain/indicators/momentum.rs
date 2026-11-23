//! Momentum: the close as a percentage of the close `period` bars earlier.
//!
//! `Momentum[i] = close[i] * 100 / close[i - period]`. Ported from
//! `Momentum.mq5` (`PRICE_CLOSE`). Values run around 100, not 0, so the
//! signal is the cross of the 100 line rather than a zero cross.
use crate::domain::cast;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, ThresholdBuffer};
use crate::validate_period;

/// Unchanged price: the line whose cross is the Momentum signal.
const MOMENTUM_LEVEL: f64 = 100.0;

pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
) -> Result<ThresholdBuffer, IndicatorError> {
    validate_period!(period, InvalidMomentumPeriod);
    let p = cast::i32_to_usize(period);

    let items = points
        .iter()
        .skip(p)
        .zip(points.iter())
        .map(|(curr, prev)| {
            // MQL5 does not guard this division; reading the level itself
            // keeps a zero close from faking a cross.
            let value = if prev.close == 0.0 {
                MOMENTUM_LEVEL
            } else {
                curr.close * 100.0 / prev.close
            };
            IndicatorValue {
                timestamp: curr.timestamp,
                value,
            }
        })
        .collect();

    Ok(ThresholdBuffer {
        items,
        high_threshold: MOMENTUM_LEVEL,
        low_threshold: MOMENTUM_LEVEL,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::indicator_values;

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
    fn constant_close_yields_level() {
        let points = points_with_closes(&[10.0; 5]);
        let result = compute(&points, 3).unwrap();
        assert_eq!(values(&result), vec![100.0, 100.0]);
        assert_eq!(timestamps(&result), vec![4, 5]);
    }

    #[test]
    fn rising_close_yields_hand_computed_momentum() {
        // 4 * 100 / 1 = 400, 5 * 100 / 2 = 250.
        let points = points_with_closes(&[1.0, 2.0, 3.0, 4.0, 5.0]);
        let result = compute(&points, 3).unwrap();
        assert_eq!(values(&result), vec![400.0, 250.0]);
    }

    #[test]
    fn falling_close_yields_momentum_below_level() {
        // 6 * 100 / 10 = 60, 4 * 100 / 8 = 50, 2 * 100 / 6 = 33.33...
        let points = points_with_closes(&[10.0, 8.0, 6.0, 4.0, 2.0]);
        let result = compute(&points, 2).unwrap();
        let momentum = values(&result);
        assert_eq!(momentum[0], 60.0);
        assert_eq!(momentum[1], 50.0);
        assert!((momentum[2] - 100.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn zero_earlier_close_yields_level() {
        // 2 * 100 / 0 is guarded to the level; 3 * 100 / 1 = 300.
        let points = points_with_closes(&[0.0, 1.0, 2.0, 3.0]);
        let result = compute(&points, 2).unwrap();
        assert_eq!(values(&result), vec![100.0, 300.0]);
    }

    #[test]
    fn both_thresholds_sit_at_level() {
        let points = points_with_closes(&[1.0, 2.0]);
        let result = compute(&points, 1).unwrap();
        assert_eq!(result.high_threshold, 100.0);
        assert_eq!(result.low_threshold, 100.0);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_items() {
        let result = compute(&[], 14).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_points_yields_empty_items() {
        let points = points_with_closes(&[10.0; 3]);
        let result = compute(&points, 3).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_plus_one_points_yields_single_value_at_last_bar() {
        let points = points_with_closes(&[10.0; 4]);
        let result = compute(&points, 3).unwrap();
        assert_eq!(timestamps(&result), vec![4]);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = points_with_closes(&[1.0, 2.0]);
        assert_eq!(
            compute(&points, 0).unwrap_err(),
            IndicatorError::InvalidMomentumPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3).unwrap_err(),
            IndicatorError::InvalidMomentumPeriod { n: -3 }
        );
    }

    // --- Level 2: real Momentum output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured by running `momentum::compute` on the
    /// frozen EURUSD fixture with `period = 14` (the MQL5 default). Not ground
    /// truth: a regression baseline to compare later against an external source.
    #[test]
    fn level2_real_momentum_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 14).unwrap();
        let momentum = indicator_values!(result);

        // 2400 points -> 2400 - 14 values.
        assert_eq!(momentum.len(), 2386);

        assert_eq!(momentum[0].timestamp, 1356307200);
        assert_eq!(momentum.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(momentum.last().expect("non-empty").value, 96.68046489412515);

        let expected_first_5: &[(i64, f64)] = &[
            (1356307200, 100.7140947798526),
            (1356480000, 101.14918784094996),
            (1356566400, 102.13859582777012),
            (1356652800, 102.23191836671542),
            (1356912000, 101.98145285935085),
        ];
        for (i, (ts, val)) in expected_first_5.iter().enumerate() {
            assert_eq!(
                momentum[i].timestamp, *ts,
                "timestamp mismatch at index {i}"
            );
            assert_eq!(momentum[i].value, *val, "value mismatch at index {i}");
        }
    }
}
