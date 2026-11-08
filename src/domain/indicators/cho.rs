//! Chaikin Oscillator: an EMA difference over the accumulation/distribution line.
//!
//! `CHO[i] = EMA(AD, fast_ma)[i] - EMA(AD, slow_ma)[i]`, where each bar
//! contributes `((close - low) - (high - close)) / (high - low) * volume`
//! to the cumulative AD. Ported from `CHO.mq5` (`VOLUME_TICK`).
use crate::domain::cast;
use crate::domain::indicators::moving_averages::ema;
use crate::domain::indicators::IndicatorError;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::{IndicatorValue, OscillatorBuffer};
use crate::empty_oscillator;
use crate::validate_period;

fn compute_ad(points: &[ResampledPointD1]) -> Vec<f64> {
    let mut ad = Vec::with_capacity(points.len());
    for pt in points {
        let range = pt.high - pt.low;
        let ad_value = if range == 0.0 {
            0.0
        } else {
            ((pt.close - pt.low) - (pt.high - pt.close)) / range * (f64::from(pt.volume))
        };
        let prev = ad.last().copied().unwrap_or(0.0);
        ad.push(prev + ad_value);
    }
    ad
}

pub fn compute(
    points: &[ResampledPointD1],
    fast_ma: i32,
    slow_ma: i32,
) -> Result<OscillatorBuffer, IndicatorError> {
    validate_period!(fast_ma, InvalidChoFastPeriod; slow_ma, InvalidChoSlowPeriod);
    if fast_ma > slow_ma {
        return Err(IndicatorError::InvalidChoPeriodOrder {
            fast: fast_ma,
            slow: slow_ma,
        });
    }
    let fast = cast::i32_to_usize(fast_ma);
    let slow = cast::i32_to_usize(slow_ma);

    let ad = compute_ad(points);
    let fast_ema = ema(&ad, fast);
    let slow_ema = ema(&ad, slow);

    if slow_ema.len() < 2 {
        empty_oscillator!();
    }

    let delta = slow.saturating_sub(fast);

    let items = slow_ema
        .iter()
        .skip(1)
        .zip(fast_ema.iter().skip(delta.saturating_add(1)))
        .zip(points.iter().skip(slow))
        .map(|((slow_val, fast_val), point)| IndicatorValue {
            timestamp: point.timestamp,
            value: fast_val - slow_val,
        })
        .collect();

    Ok(OscillatorBuffer { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::ResampledPointD1;
    use crate::indicator_values;

    /// Build a resampled point with explicit high, low, close, and volume
    /// (the four components the AD computation consumes).
    fn rp(id: i32, ts: i64, high: f64, low: f64, close: f64, volume: i32) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: "EURUSD".to_string(),
            open: close,
            high,
            low,
            close,
            volume,
            timestamp: ts,
        }
    }

    // --- synthetic hand-computed tests ---

    #[test]
    fn constant_positive_ad_yields_hand_computed_cho() {
        // All bars: high=3, low=1, close=2.5, volume=100, fast_ma=2, slow_ma=4.
        // AD value per bar = ((2.5-1)-(3-2.5))/(3-1)*100 = (1.5-0.5)/2*100 = 50.
        // Cumulative AD = [50, 100, 150, 200, 250].
        // FastEMA (period 2, k=2/3): seed=SMA(50,100)=75 at index 1;
        //   EMA[2]=(150-75)*(2/3)+75=125; EMA[3]=(200-125)*(2/3)+125=175;
        //   EMA[4]=(250-175)*(2/3)+175=225.
        // SlowEMA (period 4, k=0.4): seed=SMA(50,100,150,200)=125 at index 3;
        //   EMA[4]=(250-125)*0.4+125=175.
        // CHO starts at index slow_ma=4: CHO[4]=FastEMA[4]-SlowEMA[4]=225-175=50.0.
        // 5 points (slow_ma+1=5) -> 1 CHO value = 50.0 at points[4].ts.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, i64::from(i + 1), 3.0, 1.0, 2.5, 100))
            .collect();
        let result = compute(&points, 2, 4).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 50.0);
        assert_eq!(vals[0].timestamp, 5);
    }

    #[test]
    fn zero_ad_when_high_equals_low_yields_zero_cho() {
        // All bars: high=low=close=10.0, volume=100, fast_ma=2, slow_ma=4.
        // high=low -> AD value = 0.0 for all bars.
        // Cumulative AD = [0, 0, 0, 0, 0].
        // FastEMA = [0, 0, 0, 0]. SlowEMA = [0, 0].
        // CHO[4] = 0.0 - 0.0 = 0.0.
        // 5 points -> 1 CHO value = 0.0 at points[4].ts.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, i64::from(i + 1), 10.0, 10.0, 10.0, 100))
            .collect();
        let result = compute(&points, 2, 4).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 0.0);
        assert_eq!(vals[0].timestamp, 5);
    }

    // --- edge cases ---

    #[test]
    fn empty_points_yields_empty_buffer() {
        let result = compute(&[], 3, 10).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn fewer_than_slow_ma_plus_one_points_yields_empty_buffer() {
        // slow_ma=4, 4 points -> slow EMA has 1 entry -> no CHO values.
        let points: Vec<ResampledPointD1> = (0..4)
            .map(|i| rp(i + 1, i64::from(i + 1), 3.0, 1.0, 2.5, 100))
            .collect();
        let result = compute(&points, 2, 4).unwrap();
        assert!(result.items.is_empty());
    }

    #[test]
    fn exactly_slow_ma_plus_one_points_yields_single_value() {
        // slow_ma=4, 5 points -> slow EMA has 2 entries -> 1 CHO value.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, i64::from(i + 1), 3.0, 1.0, 2.5, 100))
            .collect();
        let result = compute(&points, 2, 4).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 1);
        assert_eq!(vals[0].value, 50.0);
        assert_eq!(vals[0].timestamp, 5);
    }

    #[test]
    fn non_positive_fast_period_returns_error() {
        let points = vec![rp(1, 1, 3.0, 1.0, 2.5, 100)];
        assert_eq!(
            compute(&points, 0, 10).unwrap_err(),
            IndicatorError::InvalidChoFastPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, -3, 10).unwrap_err(),
            IndicatorError::InvalidChoFastPeriod { n: -3 }
        );
    }

    #[test]
    fn non_positive_slow_period_returns_error() {
        let points = vec![rp(1, 1, 3.0, 1.0, 2.5, 100)];
        assert_eq!(
            compute(&points, 3, 0).unwrap_err(),
            IndicatorError::InvalidChoSlowPeriod { n: 0 }
        );
        assert_eq!(
            compute(&points, 3, -5).unwrap_err(),
            IndicatorError::InvalidChoSlowPeriod { n: -5 }
        );
    }

    #[test]
    fn timestamps_track_evaluated_bar() {
        // fast_ma=2, slow_ma=3, 5 points -> slow EMA has 3 entries -> 2 CHO
        // values at original indices 3 and 4 -> timestamps points[3].ts,
        // points[4].ts.
        let points: Vec<ResampledPointD1> = (0..5)
            .map(|i| rp(i + 1, i64::from(i + 100), 3.0, 1.0, 2.5, 100))
            .collect();
        let result = compute(&points, 2, 3).unwrap();
        let vals = indicator_values!(result);
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0].timestamp, 103);
        assert_eq!(vals[1].timestamp, 104);
    }

    // --- Level 2: real CHO output on real EURUSD data (placeholder baseline) ---

    /// Placeholder baseline captured from real EURUSD data (frozen fixture in `test_support`).
    ///
    /// These values are **not ground-truth**; they were produced by running
    /// `cho::compute` on the 2400 real EURUSD `resampled_point_d1` rows with
    /// `fast_ma = 3` and `slow_ma = 10` (the MQL5 defaults) and capturing the
    /// output. They serve as a regression baseline. The user will later
    /// compare them against an external source of truth.
    #[test]
    fn level2_real_cho_output_matches_captured_baseline() {
        let points = test_support::eurusd_points();
        let result = compute(&points, 3, 10).unwrap();
        let vals = indicator_values!(result);

        // 2400 points -> 2390 CHO values (2400 - 10).
        assert_eq!(vals.len(), 2390);

        // First and last timestamps (captured from real EURUSD data).
        assert_eq!(vals[0].timestamp, 1355788800);
        assert_eq!(vals.last().expect("non-empty").timestamp, 1646352000);

        // First 15 (timestamp, value) pairs captured from real EURUSD data.
        let expected_first_15: &[(i64, f64)] = &[
            (1355788800, 60766.70101710621),
            (1355875200, 40824.51136550082),
            (1355961600, 28556.485920767875),
            (1356048000, 14371.35629118378),
            (1356307200, 2421.8975570085386),
            (1356480000, -2111.261661974917),
            (1356566400, -6828.8181610760585),
            (1356652800, -6934.8805324285495),
            (1356912000, -8704.037048980033),
            (1356998400, -8764.890945061612),
            (1357084800, -22579.43075443337),
            (1357171200, -39904.35438919536),
            (1357257600, -34830.04565714371),
            (1357516800, -14726.830312164915),
            (1357603200, -12032.810867211956),
        ];

        for (i, (ts, val)) in expected_first_15.iter().enumerate() {
            assert_eq!(vals[i].timestamp, *ts, "timestamp mismatch at index {i}");
            assert_eq!(vals[i].value, *val, "value mismatch at index {i}");
        }
    }
}
