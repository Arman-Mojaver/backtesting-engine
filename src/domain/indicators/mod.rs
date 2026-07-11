pub mod ac;
pub mod ao;
pub mod bears;
pub mod bulls;
pub mod cho;
pub mod chv;
pub mod dpo;
pub mod force;
pub mod moving_averages;
pub mod rsi;

#[cfg(test)]
pub mod test_support;

use crate::domain::indicator::IndicatorParameters;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::IndicatorValues;

/// Which price component of a resampled point an indicator consumes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PriceField {
    Open,
    High,
    Low,
    Close,
}

impl PriceField {
    pub fn select(&self, point: &ResampledPointD1) -> f64 {
        match self {
            PriceField::Open => point.open,
            PriceField::High => point.high,
            PriceField::Low => point.low,
            PriceField::Close => point.close,
        }
    }
}

/// Domain error raised when indicator computation cannot proceed.
#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorError {
    /// The RSI period `n` is not positive.
    InvalidRsiPeriod { n: i32 },
    /// The Bears Power period `n` is not positive.
    InvalidBearsPeriod { n: i32 },
    /// The Bulls Power period `n` is not positive.
    InvalidBullsPeriod { n: i32 },
    /// The Chaikin Oscillator fast MA period `n` is not positive.
    InvalidChoFastPeriod { n: i32 },
    /// The Chaikin Oscillator slow MA period `n` is not positive.
    InvalidChoSlowPeriod { n: i32 },
    /// The Chaikin Volatility smooth period `n` is not positive.
    InvalidChvSmoothPeriod { n: i32 },
    /// The Chaikin Volatility rate-of-change period `n` is not positive.
    InvalidChvPeriod { n: i32 },
    /// The Detrended Price Oscillator period `n` is not positive.
    InvalidDpoPeriod { n: i32 },
    /// The Force Index period `n` is not positive.
    InvalidForcePeriod { n: i32 },
    /// The number of resampled points is too few for the requested window.
    NotEnoughData { points: usize, required: usize },
    /// The indicator variant has no implementation yet (e.g. MACD).
    UnsupportedIndicator { indicator: String },
}

/// Compute the indicator value buffer for the given parameters from the
/// resampled points. Dispatches off the `IndicatorParameters` variant, which
/// is both the type tag and the typed parameter bundle.
pub fn generate_indicator_values(
    parameters: &IndicatorParameters,
    points: &[ResampledPointD1],
) -> Result<IndicatorValues, IndicatorError> {
    match parameters {
        IndicatorParameters::Rsi {
            n,
            high_threshold,
            low_threshold,
        } => {
            let buffer = rsi::compute(
                points,
                PriceField::Close,
                *n,
                f64::from(*high_threshold),
                f64::from(*low_threshold),
            )?;
            Ok(IndicatorValues::OneThreshold(buffer))
        }
        IndicatorParameters::Macd { .. } => Err(IndicatorError::UnsupportedIndicator {
            indicator: "macd".to_string(),
        }),
        IndicatorParameters::AwesomeOscillator => {
            let buffer = ao::compute(points)?;
            Ok(IndicatorValues::One(buffer))
        }
        IndicatorParameters::AcceleratorDecelerator => {
            let buffer = ac::compute(points)?;
            Ok(IndicatorValues::One(buffer))
        }
        IndicatorParameters::BearsPower { period } => {
            let buffer = bears::compute(points, *period)?;
            Ok(IndicatorValues::One(buffer))
        }
        IndicatorParameters::BullsPower { period } => {
            let buffer = bulls::compute(points, *period)?;
            Ok(IndicatorValues::One(buffer))
        }
        IndicatorParameters::ChaikinOscillator { fast_ma, slow_ma } => {
            let buffer = cho::compute(points, *fast_ma, *slow_ma)?;
            Ok(IndicatorValues::One(buffer))
        }
        IndicatorParameters::ChaikinVolatility {
            smooth_period,
            chv_period,
        } => {
            let buffer = chv::compute(points, *smooth_period, *chv_period)?;
            Ok(IndicatorValues::One(buffer))
        }
        IndicatorParameters::DetrendedPriceOscillator { period } => {
            let buffer = dpo::compute(points, *period)?;
            Ok(IndicatorValues::One(buffer))
        }
        IndicatorParameters::ForceIndex { period } => {
            let buffer = force::compute(points, *period)?;
            Ok(IndicatorValues::One(buffer))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicator::IndicatorParameters;
    use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};
    use crate::domain::signal::IndicatorValues;

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
            high_low_order: HighLowOrder::HighFirst,
        }
    }

    /// 34 points with constant median -> exactly one AO value.
    fn rp_ao_34() -> Vec<ResampledPointD1> {
        (0..34)
            .map(|i| {
                let m = 1.0;
                ResampledPointD1 {
                    id: i as i32 + 1,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m + 0.5,
                    low: m - 0.5,
                    close: m,
                    volume: 1000,
                    timestamp: (i + 1) as i64,
                    high_low_order: HighLowOrder::HighFirst,
                }
            })
            .collect()
    }

    #[test]
    fn rsi_dispatch_returns_one_threshold_variant() {
        let params = IndicatorParameters::Rsi {
            n: 2,
            high_threshold: 70,
            low_threshold: 30,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0), rp(3, 3, 3.0), rp(4, 4, 4.0)];
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::OneThreshold(ref buf) => {
                assert_eq!(buf.high_threshold, 70.0);
                assert_eq!(buf.low_threshold, 30.0);
                assert!(!buf.items.is_empty());
            }
            other => panic!("expected OneThreshold, got {other:?}"),
        }
    }

    #[test]
    fn macd_dispatch_returns_unsupported_error() {
        let params = IndicatorParameters::Macd {
            slow_type: "ema".to_string(),
            slow_n: 26,
            fast_type: "ema".to_string(),
            fast_n: 12,
        };
        let points = vec![rp(1, 1, 1.0)];
        let err = generate_indicator_values(&params, &points).unwrap_err();
        assert_eq!(
            err,
            IndicatorError::UnsupportedIndicator {
                indicator: "macd".to_string()
            }
        );
    }

    #[test]
    fn awesome_oscillator_dispatch_returns_one_variant() {
        let params = IndicatorParameters::AwesomeOscillator;
        let points = rp_ao_34();
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => {
                // 34 points -> 1 AO value (constant median -> 0.0).
                assert_eq!(buf.items.len(), 1);
                assert_eq!(buf.items[0].value, 0.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn awesome_oscillator_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::AwesomeOscillator;
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn price_field_select_picks_correct_price_component() {
        let point = ResampledPointD1 {
            id: 1,
            instrument: "EURUSD".into(),
            open: 1.0,
            high: 2.0,
            low: 0.5,
            close: 1.5,
            volume: 100,
            timestamp: 1,
            high_low_order: HighLowOrder::HighFirst,
        };
        assert_eq!(PriceField::Open.select(&point), 1.0);
        assert_eq!(PriceField::High.select(&point), 2.0);
        assert_eq!(PriceField::Low.select(&point), 0.5);
        assert_eq!(PriceField::Close.select(&point), 1.5);
    }

    #[test]
    fn rsi_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Rsi {
            n: 0,
            high_threshold: 70,
            low_threshold: 30,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = generate_indicator_values(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidRsiPeriod { n: 0 });
    }

    /// 38 points with constant median -> exactly one AC value.
    fn rp_ac_38() -> Vec<ResampledPointD1> {
        (0..38)
            .map(|i| {
                let m = 1.0;
                ResampledPointD1 {
                    id: i as i32 + 1,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m + 0.5,
                    low: m - 0.5,
                    close: m,
                    volume: 1000,
                    timestamp: (i + 1) as i64,
                    high_low_order: HighLowOrder::HighFirst,
                }
            })
            .collect()
    }

    #[test]
    fn accelerator_decelerator_dispatch_returns_one_variant() {
        let params = IndicatorParameters::AcceleratorDecelerator;
        let points = rp_ac_38();
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => {
                // 38 points -> 1 AC value (constant median -> 0.0).
                assert_eq!(buf.items.len(), 1);
                assert_eq!(buf.items[0].value, 0.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn accelerator_decelerator_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::AcceleratorDecelerator;
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    /// 14 points with constant close/low -> exactly one Bears value
    /// (period 13 -> needs 14 points).
    fn rp_bears_14() -> Vec<ResampledPointD1> {
        (0..14)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i + 1,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m,
                    low: 9.0,
                    close: m,
                    volume: 1000,
                    timestamp: (i + 1) as i64,
                    high_low_order: HighLowOrder::HighFirst,
                }
            })
            .collect()
    }

    #[test]
    fn bears_power_dispatch_returns_one_variant() {
        let params = IndicatorParameters::BearsPower { period: 13 };
        let points = rp_bears_14();
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => {
                // 14 points, period 13 -> 1 Bears value (9.0 - 10.0 = -1.0).
                assert_eq!(buf.items.len(), 1);
                assert_eq!(buf.items[0].value, -1.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn bears_power_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::BearsPower { period: 13 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn bears_power_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::BearsPower { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = generate_indicator_values(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidBearsPeriod { n: 0 });
    }
    /// 14 points with constant close/high -> exactly one Bulls value
    /// (period 13 -> needs 14 points).
    fn rp_bulls_14() -> Vec<ResampledPointD1> {
        (0..14)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i + 1,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: 11.0,
                    low: m,
                    close: m,
                    volume: 1000,
                    timestamp: (i + 1) as i64,
                    high_low_order: HighLowOrder::HighFirst,
                }
            })
            .collect()
    }

    #[test]
    fn bulls_power_dispatch_returns_one_variant() {
        let params = IndicatorParameters::BullsPower { period: 13 };
        let points = rp_bulls_14();
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => {
                // 14 points, period 13 -> 1 Bulls value (11.0 - 10.0 = 1.0).
                assert_eq!(buf.items.len(), 1);
                assert_eq!(buf.items[0].value, 1.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn bulls_power_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::BullsPower { period: 13 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn bulls_power_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::BullsPower { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = generate_indicator_values(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidBullsPeriod { n: 0 });
    }

    /// 5 points with constant positive AD -> exactly one CHO value
    /// (slow_ma=4 -> needs 5 points).
    fn rp_cho_5() -> Vec<ResampledPointD1> {
        (0..5)
            .map(|i| ResampledPointD1 {
                id: i + 1,
                instrument: "EURUSD".to_string(),
                open: 2.5,
                high: 3.0,
                low: 1.0,
                close: 2.5,
                volume: 100,
                timestamp: (i + 1) as i64,
                high_low_order: HighLowOrder::HighFirst,
            })
            .collect()
    }

    #[test]
    fn chaikin_oscillator_dispatch_returns_one_variant() {
        let params = IndicatorParameters::ChaikinOscillator {
            fast_ma: 2,
            slow_ma: 4,
        };
        let points = rp_cho_5();
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => {
                // 5 points, slow_ma=4 -> 1 CHO value = 50.0.
                assert_eq!(buf.items.len(), 1);
                assert_eq!(buf.items[0].value, 50.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn chaikin_oscillator_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::ChaikinOscillator {
            fast_ma: 3,
            slow_ma: 10,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn chaikin_oscillator_invalid_fast_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::ChaikinOscillator {
            fast_ma: 0,
            slow_ma: 10,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = generate_indicator_values(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidChoFastPeriod { n: 0 });
    }

    #[test]
    fn chaikin_oscillator_invalid_slow_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::ChaikinOscillator {
            fast_ma: 3,
            slow_ma: 0,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = generate_indicator_values(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidChoSlowPeriod { n: 0 });
    }

    /// 5 points with constant high-low spread -> exactly one CHV value
    /// (smooth_period=2, chv_period=3 -> needs 5 points).
    fn rp_chv_5() -> Vec<ResampledPointD1> {
        (0..5)
            .map(|i| ResampledPointD1 {
                id: i + 1,
                instrument: "EURUSD".to_string(),
                open: 9.0,
                high: 11.0,
                low: 9.0,
                close: 11.0,
                volume: 1000,
                timestamp: (i + 1) as i64,
                high_low_order: HighLowOrder::HighFirst,
            })
            .collect()
    }

    #[test]
    fn chaikin_volatility_dispatch_returns_one_variant() {
        let params = IndicatorParameters::ChaikinVolatility {
            smooth_period: 2,
            chv_period: 3,
        };
        let points = rp_chv_5();
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => {
                // 5 points, smooth=2, chv=3 -> 1 CHV value = 0.0 (constant spread).
                assert_eq!(buf.items.len(), 1);
                assert_eq!(buf.items[0].value, 0.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn chaikin_volatility_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::ChaikinVolatility {
            smooth_period: 10,
            chv_period: 10,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn chaikin_volatility_invalid_smooth_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::ChaikinVolatility {
            smooth_period: 0,
            chv_period: 10,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = generate_indicator_values(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidChvSmoothPeriod { n: 0 });
    }

    #[test]
    fn chaikin_volatility_invalid_chv_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::ChaikinVolatility {
            smooth_period: 10,
            chv_period: 0,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = generate_indicator_values(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidChvPeriod { n: 0 });
    }

    /// 7 points with constant close -> exactly one DPO value
    /// (period 12 -> ExtMAPeriod = 7 -> needs 7 points).
    fn rp_dpo_7() -> Vec<ResampledPointD1> {
        (0..7)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i + 1,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m,
                    low: m,
                    close: m,
                    volume: 1000,
                    timestamp: (i + 1) as i64,
                    high_low_order: HighLowOrder::HighFirst,
                }
            })
            .collect()
    }

    #[test]
    fn detrended_price_oscillator_dispatch_returns_one_variant() {
        let params = IndicatorParameters::DetrendedPriceOscillator { period: 12 };
        let points = rp_dpo_7();
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => {
                // 7 points, period 12 -> ExtMAPeriod = 7 -> 1 DPO value
                // (10.0 - 10.0 = 0.0).
                assert_eq!(buf.items.len(), 1);
                assert_eq!(buf.items[0].value, 0.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn detrended_price_oscillator_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::DetrendedPriceOscillator { period: 12 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn detrended_price_oscillator_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::DetrendedPriceOscillator { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = generate_indicator_values(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidDpoPeriod { n: 0 });
    }

    /// 4 points with constant close -> exactly one Force Index value
    /// (period 3 -> needs 4 points).
    fn rp_force_4() -> Vec<ResampledPointD1> {
        (0..4)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i + 1,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m,
                    low: m,
                    close: m,
                    volume: 1000,
                    timestamp: (i + 1) as i64,
                    high_low_order: HighLowOrder::HighFirst,
                }
            })
            .collect()
    }

    #[test]
    fn force_index_dispatch_returns_one_variant() {
        let params = IndicatorParameters::ForceIndex { period: 3 };
        let points = rp_force_4();
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => {
                // 4 points, period 3 -> 1 Force Index value
                // (1000 * (10.0 - 10.0) = 0.0).
                assert_eq!(buf.items.len(), 1);
                assert_eq!(buf.items[0].value, 0.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn force_index_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::ForceIndex { period: 13 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = generate_indicator_values(&params, &points).unwrap();
        match result {
            IndicatorValues::One(ref buf) => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn force_index_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::ForceIndex { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = generate_indicator_values(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidForcePeriod { n: 0 });
    }
}
