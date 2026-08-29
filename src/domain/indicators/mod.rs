pub mod ac;
pub mod ao;
pub mod bears;
pub mod bulls;
pub mod cho;
pub mod chv;
pub mod dpo;
pub mod force;
pub mod macros;
pub mod moving_averages;
pub mod osma;
pub mod roc;
pub mod rsi;

#[cfg(test)]
pub mod test_support;
pub mod trix;
pub mod vroc;

use crate::domain::indicator::IndicatorParameters;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::IndicatorOutput;

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

#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorError {
    InvalidRsiPeriod { n: i32 },
    InvalidBearsPeriod { n: i32 },
    InvalidBullsPeriod { n: i32 },
    InvalidChoFastPeriod { n: i32 },
    InvalidChoSlowPeriod { n: i32 },
    InvalidChvSmoothPeriod { n: i32 },
    InvalidChvPeriod { n: i32 },
    InvalidDpoPeriod { n: i32 },
    InvalidForcePeriod { n: i32 },
    InvalidOsMaFastPeriod { n: i32 },
    InvalidOsMaSlowPeriod { n: i32 },
    InvalidOsMaSignalPeriod { n: i32 },
    InvalidRocPeriod { n: i32 },
    InvalidTrixPeriod { n: i32 },
    InvalidVrocPeriod { n: i32 },
    NotEnoughData { points: usize, required: usize },
    UnsupportedIndicator { indicator: String },
}

pub fn create_indicator_output(
    parameters: &IndicatorParameters,
    points: &[ResampledPointD1],
) -> Result<IndicatorOutput, IndicatorError> {
    let instrument = points.first().map(|p| p.instrument.clone()).unwrap();
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
            Ok(IndicatorOutput::Threshold { instrument, buffer })
        }
        IndicatorParameters::Macd { .. } => Err(IndicatorError::UnsupportedIndicator {
            indicator: "macd".to_string(),
        }),
        IndicatorParameters::AwesomeOscillator => {
            let buffer = ao::compute(points)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::AcceleratorDecelerator => {
            let buffer = ac::compute(points)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::BearsPower { period } => {
            let buffer = bears::compute(points, *period)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::BullsPower { period } => {
            let buffer = bulls::compute(points, *period)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::ChaikinOscillator { fast_ma, slow_ma } => {
            let buffer = cho::compute(points, *fast_ma, *slow_ma)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::ChaikinVolatility {
            smooth_period,
            chv_period,
        } => {
            let buffer = chv::compute(points, *smooth_period, *chv_period)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::DetrendedPriceOscillator { period } => {
            let buffer = dpo::compute(points, *period)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::ForceIndex { period } => {
            let buffer = force::compute(points, *period)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::OsMa {
            fast_ema,
            slow_ema,
            signal_sma,
        } => {
            let buffer = osma::compute(points, *fast_ema, *slow_ema, *signal_sma)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::Roc { period } => {
            let buffer = roc::compute(points, *period)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::Trix { period } => {
            let buffer = trix::compute(points, *period)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::Vroc { period } => {
            let buffer = vroc::compute(points, *period)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicator::IndicatorParameters;
    use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};
    use crate::domain::signal::IndicatorOutput;

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
                    id: i + 1,
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Threshold {
                buffer: ref buf, ..
            } => {
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
        let err = create_indicator_output(&params, &points).unwrap_err();
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
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
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidRsiPeriod { n: 0 });
    }

    /// 38 points with constant median -> exactly one AC value.
    fn rp_ac_38() -> Vec<ResampledPointD1> {
        (0..38)
            .map(|i| {
                let m = 1.0;
                ResampledPointD1 {
                    id: i + 1,
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn bears_power_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::BearsPower { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn bulls_power_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::BullsPower { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
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
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidChoFastPeriod { n: 0 });
    }

    #[test]
    fn chaikin_oscillator_invalid_slow_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::ChaikinOscillator {
            fast_ma: 3,
            slow_ma: 0,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
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
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidChvSmoothPeriod { n: 0 });
    }

    #[test]
    fn chaikin_volatility_invalid_chv_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::ChaikinVolatility {
            smooth_period: 10,
            chv_period: 0,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn detrended_price_oscillator_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::DetrendedPriceOscillator { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
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
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn force_index_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::ForceIndex { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidForcePeriod { n: 0 });
    }

    /// 6 points with constant close -> exactly one OsMA value
    /// (fast=3, slow=5, signal=2 -> needs 6 points).
    fn rp_osma_6() -> Vec<ResampledPointD1> {
        (0..6)
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
    fn osma_dispatch_returns_one_variant() {
        let params = IndicatorParameters::OsMa {
            fast_ema: 3,
            slow_ema: 5,
            signal_sma: 2,
        };
        let points = rp_osma_6();
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
                // 6 points, fast=3, slow=5, signal=2 -> 1 OsMA value
                // (constant close -> 0.0).
                assert_eq!(buf.items.len(), 1);
                assert_eq!(buf.items[0].value, 0.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn osma_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::OsMa {
            fast_ema: 12,
            slow_ema: 26,
            signal_sma: 9,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn osma_invalid_fast_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::OsMa {
            fast_ema: 0,
            slow_ema: 26,
            signal_sma: 9,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidOsMaFastPeriod { n: 0 });
    }

    #[test]
    fn osma_invalid_slow_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::OsMa {
            fast_ema: 12,
            slow_ema: 0,
            signal_sma: 9,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidOsMaSlowPeriod { n: 0 });
    }

    #[test]
    fn osma_invalid_signal_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::OsMa {
            fast_ema: 12,
            slow_ema: 26,
            signal_sma: 0,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidOsMaSignalPeriod { n: 0 });
    }

    /// 5 points with constant close -> 2 ROC values
    /// (period 3 -> needs 4 points, 5 points -> 2 ROC values).
    fn rp_roc_5() -> Vec<ResampledPointD1> {
        (0..5)
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
    fn roc_dispatch_returns_one_variant() {
        let params = IndicatorParameters::Roc { period: 3 };
        let points = rp_roc_5();
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
                // 5 points, period 3 -> 2 ROC values (constant close -> 0.0).
                assert_eq!(buf.items.len(), 2);
                assert_eq!(buf.items[0].value, 0.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn roc_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::Roc { period: 13 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn roc_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Roc { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidRocPeriod { n: 0 });
    }

    /// 5 points with constant close -> exactly 1 TRIX value
    /// (period 2 -> needs 5 points = 3*2 - 1).
    fn rp_trix_5() -> Vec<ResampledPointD1> {
        (0..5)
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
    fn trix_dispatch_returns_one_variant() {
        let params = IndicatorParameters::Trix { period: 2 };
        let points = rp_trix_5();
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
                // 5 points, period 2 -> 1 TRIX value (constant close -> 0.0).
                assert_eq!(buf.items.len(), 1);
                assert_eq!(buf.items[0].value, 0.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn trix_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::Trix { period: 14 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn trix_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Trix { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidTrixPeriod { n: 0 });
    }

    /// 5 points with constant volume -> 4 VROC values
    /// (period 2 -> lookback 1 -> needs 2 points, 5 points -> 4 VROC values).
    fn rp_vroc_5() -> Vec<ResampledPointD1> {
        (0..5)
            .map(|i| {
                let v = 1000;
                ResampledPointD1 {
                    id: i + 1,
                    instrument: "EURUSD".to_string(),
                    open: 10.0,
                    high: 10.0,
                    low: 10.0,
                    close: 10.0,
                    volume: v,
                    timestamp: (i + 1) as i64,
                    high_low_order: HighLowOrder::HighFirst,
                }
            })
            .collect()
    }

    #[test]
    fn vroc_dispatch_returns_one_variant() {
        let params = IndicatorParameters::Vroc { period: 2 };
        let points = rp_vroc_5();
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
                // 5 points, period 2 -> 4 VROC values (constant volume -> 0.0).
                assert_eq!(buf.items.len(), 4);
                assert_eq!(buf.items[0].value, 0.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn vroc_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::Vroc { period: 25 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => assert!(buf.items.is_empty()),
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn vroc_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Vroc { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidVrocPeriod { n: 0 });
    }
}
