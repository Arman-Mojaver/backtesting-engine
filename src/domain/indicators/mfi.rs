//! Money Flow Index (MFI): a volume-weighted RSI over the typical price.
//!
//! `TP = (high + low + close) / 3` and each bar's raw money flow is
//! `volume * TP`, counted as positive when TP rose against the previous bar
//! and negative when it fell. `MFI = 100 - 100 / (1 + positive / negative)`
//! over the last `period` flows. Ported from `MFI.mq5` (`VOLUME_TICK`).
use crate::domain::cast;
use crate::domain::indicators::{IndicatorError, typical_price};
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, ThresholdBuffer};
use crate::validate_period;

#[derive(Debug, Clone, Copy)]
struct MoneyFlow {
    timestamp: i64,
    positive: f64,
    negative: f64,
}

pub fn compute(
    points: &[ResampledPointD1],
    period: i32,
    high_threshold: i32,
    low_threshold: i32,
) -> Result<ThresholdBuffer, IndicatorError> {
    validate_period!(period, InvalidMfiPeriod);

    for threshold in [high_threshold, low_threshold] {
        if !(0..=100).contains(&threshold) {
            return Err(IndicatorError::InvalidMfiThresholdRange { value: threshold });
        }
    }

    if low_threshold >= high_threshold {
        return Err(IndicatorError::InvalidMfiThresholdOrder {
            high: high_threshold,
            low: low_threshold,
        });
    }

    let p = cast::i32_to_usize(period);

    let flows: Vec<MoneyFlow> = points
        .iter()
        .zip(points.iter().skip(1))
        .map(|(prev, cur)| {
            let prev_tp = typical_price(prev);
            let cur_tp = typical_price(cur);
            let raw_flow = f64::from(cur.volume) * cur_tp;
            MoneyFlow {
                timestamp: cur.timestamp,
                positive: if cur_tp > prev_tp { raw_flow } else { 0.0 },
                negative: if cur_tp < prev_tp { raw_flow } else { 0.0 },
            }
        })
        .collect();

    Ok(ThresholdBuffer {
        items: flows.windows(p).map(compute_mfi_point).collect(),
        high_threshold: f64::from(high_threshold),
        low_threshold: f64::from(low_threshold),
    })
}

fn compute_mfi_point(window: &[MoneyFlow]) -> IndicatorValue {
    let mut positive: f64 = 0.0;
    let mut negative: f64 = 0.0;
    let mut timestamp: i64 = 0;
    for flow in window {
        positive += flow.positive;
        negative += flow.negative;
        timestamp = flow.timestamp;
    }

    // MQL5 reports 100 whenever no money flowed out, a flat window included.
    let value = if negative == 0.0 {
        100.0
    } else {
        100.0 - 100.0 / (1.0 + positive / negative)
    };

    IndicatorValue { timestamp, value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::ResampledPointD1;
    use crate::indicator_values;

    /// High, low and close all equal `price`, so the typical price is `price`.
    fn rp(id: i32, ts: i64, price: f64, volume: i32) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: price,
            high: price,
            low: price,
            close: price,
            volume,
            timestamp: ts,
        }
    }

    fn points_with_prices(prices: &[f64], volume: i32) -> Vec<ResampledPointD1> {
        prices
            .iter()
            .zip(1..)
            .map(|(&price, i)| rp(i, i64::from(i), price, volume))
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
    fn rising_typical_price_yields_max_mfi() {
        let points = points_with_prices(&[1.0, 2.0, 3.0, 4.0, 5.0], 100);
        let result = compute(&points, 3, 80, 20).unwrap();
        assert_eq!(values(&result), vec![100.0, 100.0]);
        assert_eq!(timestamps(&result), vec![4, 5]);
    }

    #[test]
    fn falling_typical_price_yields_min_mfi() {
        let points = points_with_prices(&[5.0, 4.0, 3.0, 2.0, 1.0], 100);
        let result = compute(&points, 3, 80, 20).unwrap();
        assert_eq!(values(&result), vec![0.0, 0.0]);
    }

    #[test]
    fn flat_typical_price_yields_max_mfi_like_mql5() {
        let points = points_with_prices(&[2.0; 4], 100);
        let result = compute(&points, 3, 80, 20).unwrap();
        assert_eq!(values(&result), vec![100.0]);
    }

    #[test]
    fn mixed_flows_yield_hand_computed_mfi() {
        // TP = [1, 2, 1, 3], volumes = [_, 10, 20, 30], period 3.
        // Flows: +10*2 = 20, -20*1 = 20, +30*3 = 90.
        // positive = 110, negative = 20 -> MFI = 100 - 100 / (1 + 5.5) = 1100/13.
        let points = vec![
            rp(1, 1, 1.0, 5),
            rp(2, 2, 2.0, 10),
            rp(3, 3, 1.0, 20),
            rp(4, 4, 3.0, 30),
        ];
        let result = compute(&points, 3, 80, 20).unwrap();
        let mfi = values(&result);
        assert_eq!(mfi.len(), 1);
        assert!((mfi[0] - 1100.0 / 13.0).abs() < 1e-12);
    }

    #[test]
    fn typical_price_averages_high_low_and_close() {
        // TP1 = (3 + 0 + 0) / 3 = 1; TP2 = (1 + 1 + 1) / 3 = 1: unchanged, not
        // rising, although the close rose from 0 to 1.
        let mut first = rp(1, 1, 0.0, 100);
        first.high = 3.0;
        let points = vec![first, rp(2, 2, 1.0, 100), rp(3, 3, 0.5, 100)];
        let result = compute(&points, 2, 80, 20).unwrap();
        assert_eq!(values(&result), vec![0.0]);
    }

    #[test]
    fn thresholds_carried_through_as_levels() {
        let points = points_with_prices(&[1.0, 2.0, 3.0], 100);
        let result = compute(&points, 2, 80, 20).unwrap();
        assert_eq!(result.high_threshold, 80.0);
        assert_eq!(result.low_threshold, 20.0);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_items() {
        let result = compute(&[], 14, 80, 20).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_period_points_yields_empty_items() {
        let points = points_with_prices(&[1.0, 2.0, 3.0], 100);
        let result = compute(&points, 3, 80, 20).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn period_plus_one_points_yields_single_value_at_last_bar() {
        let points = points_with_prices(&[1.0, 2.0, 3.0, 4.0], 100);
        let result = compute(&points, 3, 80, 20).unwrap();
        assert_eq!(timestamps(&result), vec![4]);
    }

    #[test]
    fn non_positive_period_returns_error() {
        let points = points_with_prices(&[1.0, 2.0], 100);
        assert_eq!(
            compute(&points, 0, 80, 20).unwrap_err(),
            IndicatorError::InvalidMfiPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3, 80, 20).unwrap_err(),
            IndicatorError::InvalidMfiPeriod { n: -3 }
        );
    }

    #[test]
    fn threshold_outside_0_100_returns_error() {
        let points = points_with_prices(&[1.0, 2.0], 100);
        assert_eq!(
            compute(&points, 2, 101, 20).unwrap_err(),
            IndicatorError::InvalidMfiThresholdRange { value: 101 }
        );
        assert_eq!(
            compute(&points, 2, 80, 101).unwrap_err(),
            IndicatorError::InvalidMfiThresholdRange { value: 101 }
        );
    }

    #[test]
    fn low_threshold_not_below_high_returns_error() {
        let points = points_with_prices(&[1.0, 2.0], 100);
        assert_eq!(
            compute(&points, 2, 20, 80).unwrap_err(),
            IndicatorError::InvalidMfiThresholdOrder { high: 20, low: 80 }
        );
        assert_eq!(
            compute(&points, 2, 50, 50).unwrap_err(),
            IndicatorError::InvalidMfiThresholdOrder { high: 50, low: 50 }
        );
    }

    // --- Level 2: real MFI output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured by running `mfi::compute` on the frozen
    /// EURUSD fixture with `period = 14` (the MQL5 default). Not ground truth:
    /// a regression baseline to compare later against an external source.
    #[test]
    fn level2_real_mfi_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 14, 80, 20).unwrap();
        let mfi = indicator_values!(result);

        // 2400 points -> 2400 - 14 values.
        assert_eq!(mfi.len(), 2386);

        assert_eq!(mfi[0].timestamp, 1356307200);
        assert_eq!(mfi.last().expect("non-empty").timestamp, 1646352000);
        assert_eq!(mfi.last().expect("non-empty").value, 19.20819284610009);

        let expected_first_5: &[(i64, f64)] = &[
            (1356307200, 69.44721932381314),
            (1356480000, 68.30945378436382),
            (1356566400, 76.25284997919461),
            (1356652800, 77.03460635255203),
            (1356912000, 78.40816680051782),
        ];
        for (i, (ts, val)) in expected_first_5.iter().enumerate() {
            assert_eq!(mfi[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(mfi[i].value, *val, "value mismatch at index {i}");
        }
    }
}
