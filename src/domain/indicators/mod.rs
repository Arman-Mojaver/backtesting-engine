pub mod ac;
pub mod ao;
pub mod bears;
pub mod bulls;
pub mod cho;
pub mod chv;
pub mod dpo;
pub mod force;
pub mod macd;
pub mod macros;
pub mod moving_averages;
pub mod osma;
pub mod roc;
pub mod rsi;

#[cfg(test)]
pub mod test_support;
pub mod trix;
pub mod vroc;

use std::error::Error;
use std::fmt;

use crate::domain::indicator::IndicatorParameters;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::IndicatorOutput;

/// Which price component of a resampled point an indicator consumes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceField {
    Open,
    High,
    Low,
    Close,
}

impl PriceField {
    pub const fn select(&self, point: &ResampledPointD1) -> f64 {
        match self {
            Self::Open => point.open,
            Self::High => point.high,
            Self::Low => point.low,
            Self::Close => point.close,
        }
    }
}

/// Typed error taxonomy for indicator computation.
///
/// Each variant names one invalid indicator configuration (which
/// indicator, which parameter, the offending value) or a data shortfall,
/// so callers can report or skip the offending row precisely instead of
/// aborting the whole run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndicatorError {
    InvalidMacdFastPeriod { n: i32 },
    InvalidMacdSlowPeriod { n: i32 },
    InvalidMacdMaType { ma_type: String },
    InvalidMacdPeriodOrder { fast: i32, slow: i32 },
    InvalidRsiPeriod { n: i32 },
    InvalidRsiThresholdOrder { high: i32, low: i32 },
    InvalidRsiThresholdRange { value: i32 },
    InvalidBearsPeriod { n: i32 },
    InvalidBullsPeriod { n: i32 },
    InvalidChoFastPeriod { n: i32 },
    InvalidChoSlowPeriod { n: i32 },
    InvalidChoPeriodOrder { fast: i32, slow: i32 },
    InvalidChvSmoothPeriod { n: i32 },
    InvalidChvPeriod { n: i32 },
    InvalidDpoPeriod { n: i32 },
    InvalidForcePeriod { n: i32 },
    InvalidOsMaFastPeriod { n: i32 },
    InvalidOsMaSlowPeriod { n: i32 },
    InvalidOsMaSignalPeriod { n: i32 },
    InvalidOsMaPeriodOrder { fast: i32, slow: i32 },
    InvalidRocPeriod { n: i32 },
    InvalidTrixPeriod { n: i32 },
    InvalidVrocPeriod { n: i32 },
    NotEnoughData { points: usize, required: usize },
}

impl fmt::Display for IndicatorError {
    // A flat, exhaustive per-variant match is the canonical shape for a
    // hand-rolled Display (it mirrors what thiserror generates); routing
    // it through helpers would hide the exact per-variant sentences.
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMacdFastPeriod { n } => {
                write!(f, "macd fast period must be positive, got {n}")
            }
            Self::InvalidMacdSlowPeriod { n } => {
                write!(f, "macd slow period must be positive, got {n}")
            }
            Self::InvalidMacdMaType { ma_type } => {
                write!(
                    f,
                    "macd moving-average type must be \"ema\" or \"sma\", got \"{ma_type}\""
                )
            }
            Self::InvalidMacdPeriodOrder { fast, slow } => {
                write!(
                    f,
                    "macd fast period must be less than slow period, got fast {fast}, slow {slow}"
                )
            }
            Self::InvalidRsiPeriod { n } => {
                write!(f, "rsi period must be positive, got {n}")
            }
            Self::InvalidRsiThresholdOrder { high, low } => {
                write!(
                    f,
                    "rsi low threshold must be less than high threshold, got high {high}, low {low}"
                )
            }
            Self::InvalidRsiThresholdRange { value } => {
                write!(f, "rsi threshold must be between 0 and 100, got {value}")
            }
            Self::InvalidBearsPeriod { n } => {
                write!(f, "bears power period must be positive, got {n}")
            }
            Self::InvalidBullsPeriod { n } => {
                write!(f, "bulls power period must be positive, got {n}")
            }
            Self::InvalidChoFastPeriod { n } => {
                write!(
                    f,
                    "chaikin oscillator fast period must be positive, got {n}"
                )
            }
            Self::InvalidChoSlowPeriod { n } => {
                write!(
                    f,
                    "chaikin oscillator slow period must be positive, got {n}"
                )
            }
            Self::InvalidChoPeriodOrder { fast, slow } => {
                write!(
                    f,
                    "chaikin oscillator fast period must not exceed slow period, got fast {fast}, slow {slow}"
                )
            }
            Self::InvalidChvSmoothPeriod { n } => {
                write!(
                    f,
                    "chaikin volatility smoothing period must be positive, got {n}"
                )
            }
            Self::InvalidChvPeriod { n } => {
                write!(f, "chaikin volatility period must be positive, got {n}")
            }
            Self::InvalidDpoPeriod { n } => {
                write!(
                    f,
                    "detrended price oscillator period must be positive, got {n}"
                )
            }
            Self::InvalidForcePeriod { n } => {
                write!(f, "force index period must be positive, got {n}")
            }
            Self::InvalidOsMaFastPeriod { n } => {
                write!(f, "osma fast period must be positive, got {n}")
            }
            Self::InvalidOsMaSlowPeriod { n } => {
                write!(f, "osma slow period must be positive, got {n}")
            }
            Self::InvalidOsMaSignalPeriod { n } => {
                write!(f, "osma signal period must be positive, got {n}")
            }
            Self::InvalidOsMaPeriodOrder { fast, slow } => {
                write!(
                    f,
                    "osma fast period must not exceed slow period, got fast {fast}, slow {slow}"
                )
            }
            Self::InvalidRocPeriod { n } => {
                write!(f, "rate of change period must be positive, got {n}")
            }
            Self::InvalidTrixPeriod { n } => {
                write!(f, "trix period must be positive, got {n}")
            }
            Self::InvalidVrocPeriod { n } => {
                write!(f, "volume rate of change period must be positive, got {n}")
            }
            Self::NotEnoughData { points, required } => {
                write!(
                    f,
                    "not enough data points to compute indicator: saw {points}, required {required}"
                )
            }
        }
    }
}

impl Error for IndicatorError {}

pub fn create_indicator_output(
    parameters: &IndicatorParameters,
    points: &[ResampledPointD1],
) -> Result<IndicatorOutput, IndicatorError> {
    let instrument =
        points
            .first()
            .map(|p| p.instrument.clone())
            .ok_or(IndicatorError::NotEnoughData {
                points: 0,
                required: 1,
            })?;
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
        IndicatorParameters::Macd {
            slow_type,
            slow_n,
            fast_type,
            fast_n,
        } => {
            let buffer = macd::compute(points, fast_type, *fast_n, slow_type, *slow_n)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
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
    use crate::domain::resampled_point::ResampledPointD1;
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
        }
    }

    /// 34 points with constant median -> exactly one AO value.
    fn rp_ao_34() -> Vec<ResampledPointD1> {
        (1..=34)
            .map(|i| {
                let m = 1.0;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m + 0.5,
                    low: m - 0.5,
                    close: m,
                    volume: 1000,
                    timestamp: i64::from(i),
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

    /// 6 points with constant close -> exactly 2 MACD values
    /// (fast=3, slow=5 -> values start at index 4).
    fn rp_macd_6() -> Vec<ResampledPointD1> {
        (1..=6)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m,
                    low: m,
                    close: m,
                    volume: 1000,
                    timestamp: i64::from(i),
                }
            })
            .collect()
    }

    #[test]
    fn macd_dispatch_returns_one_variant() {
        let params = IndicatorParameters::Macd {
            slow_type: "ema".to_string(),
            slow_n: 5,
            fast_type: "ema".to_string(),
            fast_n: 3,
        };
        let points = rp_macd_6();
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Oscillator {
                buffer: ref buf, ..
            } => {
                // 6 points, fast=3, slow=5 -> 2 MACD values (constant close -> 0.0).
                assert_eq!(buf.items.len(), 2);
                assert_eq!(buf.items[0].value, 0.0);
            }
            other => panic!("expected One, got {other:?}"),
        }
    }

    #[test]
    fn macd_dispatch_with_insufficient_data_yields_empty_one() {
        let params = IndicatorParameters::Macd {
            slow_type: "ema".to_string(),
            slow_n: 26,
            fast_type: "ema".to_string(),
            fast_n: 12,
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
    fn macd_dispatch_invalid_ma_type_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Macd {
            slow_type: "ema".to_string(),
            slow_n: 26,
            fast_type: "wma".to_string(),
            fast_n: 12,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(
            err,
            IndicatorError::InvalidMacdMaType {
                ma_type: "wma".to_string()
            }
        );
    }

    #[test]
    fn macd_invalid_fast_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Macd {
            slow_type: "ema".to_string(),
            slow_n: 26,
            fast_type: "ema".to_string(),
            fast_n: 0,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidMacdFastPeriod { n: 0 });
    }

    #[test]
    fn macd_invalid_slow_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Macd {
            slow_type: "ema".to_string(),
            slow_n: 0,
            fast_type: "ema".to_string(),
            fast_n: 12,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidMacdSlowPeriod { n: 0 });
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
        (1..=38)
            .map(|i| {
                let m = 1.0;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m + 0.5,
                    low: m - 0.5,
                    close: m,
                    volume: 1000,
                    timestamp: i64::from(i),
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
        (1..=14)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m,
                    low: 9.0,
                    close: m,
                    volume: 1000,
                    timestamp: i64::from(i),
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
        (1..=14)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: 11.0,
                    low: m,
                    close: m,
                    volume: 1000,
                    timestamp: i64::from(i),
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
        (1..=5)
            .map(|i| ResampledPointD1 {
                id: i,
                instrument: "EURUSD".to_string(),
                open: 2.5,
                high: 3.0,
                low: 1.0,
                close: 2.5,
                volume: 100,
                timestamp: i64::from(i),
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
        (1..=5)
            .map(|i| ResampledPointD1 {
                id: i,
                instrument: "EURUSD".to_string(),
                open: 9.0,
                high: 11.0,
                low: 9.0,
                close: 11.0,
                volume: 1000,
                timestamp: i64::from(i),
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
        (1..=7)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m,
                    low: m,
                    close: m,
                    volume: 1000,
                    timestamp: i64::from(i),
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
        (1..=4)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m,
                    low: m,
                    close: m,
                    volume: 1000,
                    timestamp: i64::from(i),
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
        (1..=6)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m,
                    low: m,
                    close: m,
                    volume: 1000,
                    timestamp: i64::from(i),
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
        (1..=5)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m,
                    low: m,
                    close: m,
                    volume: 1000,
                    timestamp: i64::from(i),
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
        (1..=5)
            .map(|i| {
                let m = 10.0;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: m,
                    high: m,
                    low: m,
                    close: m,
                    volume: 1000,
                    timestamp: i64::from(i),
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
        (1..=5)
            .map(|i| {
                let v = 1000;
                ResampledPointD1 {
                    id: i,
                    instrument: "EURUSD".to_string(),
                    open: 10.0,
                    high: 10.0,
                    low: 10.0,
                    close: 10.0,
                    volume: v,
                    timestamp: i64::from(i),
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

    // One table asserting every variant's Display output; splitting it
    // to satisfy the line-count lint would scatter one concern across fns.
    #[test]
    #[allow(clippy::too_many_lines)]
    fn display_messages_are_operator_ready_sentences() {
        let cases = [
            (
                IndicatorError::InvalidMacdFastPeriod { n: 0 },
                "macd fast period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidMacdSlowPeriod { n: 0 },
                "macd slow period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidMacdMaType {
                    ma_type: "wma".to_string(),
                },
                "macd moving-average type must be \"ema\" or \"sma\", got \"wma\"",
            ),
            (
                IndicatorError::InvalidMacdPeriodOrder { fast: 4, slow: 2 },
                "macd fast period must be less than slow period, got fast 4, slow 2",
            ),
            (
                IndicatorError::InvalidRsiPeriod { n: 0 },
                "rsi period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidRsiThresholdOrder { high: 30, low: 40 },
                "rsi low threshold must be less than high threshold, got high 30, low 40",
            ),
            (
                IndicatorError::InvalidRsiThresholdRange { value: 101 },
                "rsi threshold must be between 0 and 100, got 101",
            ),
            (
                IndicatorError::InvalidBearsPeriod { n: 0 },
                "bears power period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidBullsPeriod { n: 0 },
                "bulls power period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidChoFastPeriod { n: 0 },
                "chaikin oscillator fast period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidChoSlowPeriod { n: 0 },
                "chaikin oscillator slow period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidChoPeriodOrder { fast: 10, slow: 4 },
                "chaikin oscillator fast period must not exceed slow period, got fast 10, slow 4",
            ),
            (
                IndicatorError::InvalidChvSmoothPeriod { n: 0 },
                "chaikin volatility smoothing period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidChvPeriod { n: 0 },
                "chaikin volatility period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidDpoPeriod { n: 0 },
                "detrended price oscillator period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidForcePeriod { n: 0 },
                "force index period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidOsMaFastPeriod { n: 0 },
                "osma fast period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidOsMaSlowPeriod { n: 0 },
                "osma slow period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidOsMaSignalPeriod { n: 0 },
                "osma signal period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidOsMaPeriodOrder { fast: 26, slow: 12 },
                "osma fast period must not exceed slow period, got fast 26, slow 12",
            ),
            (
                IndicatorError::InvalidRocPeriod { n: 0 },
                "rate of change period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidTrixPeriod { n: 0 },
                "trix period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidVrocPeriod { n: 0 },
                "volume rate of change period must be positive, got 0",
            ),
            (
                IndicatorError::NotEnoughData {
                    points: 2,
                    required: 26,
                },
                "not enough data points to compute indicator: saw 2, required 26",
            ),
        ];

        for (error, expected) in cases {
            assert_eq!(
                error.to_string(),
                expected,
                "Display mismatch for {error:?}"
            );
        }
    }

    #[test]
    fn indicator_error_is_a_std_error_object() {
        let boxed: Box<dyn std::error::Error> =
            Box::new(IndicatorError::InvalidTrixPeriod { n: 0 });
        assert_eq!(boxed.to_string(), "trix period must be positive, got 0");
        assert!(boxed.source().is_none());
    }
}
