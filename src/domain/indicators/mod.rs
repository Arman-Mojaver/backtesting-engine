//! Ports of the MQL5 indicator sources under `Indicators/`.
//!
//! Warm-up bars produce no entries and too little data yields an empty
//! buffer, never an error; a non-positive period is the only configuration
//! error.
pub mod ac;
pub mod ao;
pub mod bears;
pub mod bulls;
pub mod cho;
pub mod chv;
pub mod dema;
pub mod dpo;
pub mod force;
pub mod macd;
pub mod macros;
pub mod mfi;
pub mod moving_averages;
pub mod osma;
pub mod roc;
pub mod rsi;
pub mod rvi;
pub mod stochastic;
pub mod tema;

#[cfg(test)]
pub mod test_support;
pub mod trix;
pub mod vroc;

use thiserror::Error;

use crate::domain::indicator::IndicatorParameters;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::IndicatorOutput;

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

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IndicatorError {
    #[error("macd fast period must be positive, got {n}")]
    InvalidMacdFastPeriod { n: i32 },
    #[error("macd slow period must be positive, got {n}")]
    InvalidMacdSlowPeriod { n: i32 },
    #[error("macd fast period must be less than slow period, got fast {fast}, slow {slow}")]
    InvalidMacdPeriodOrder { fast: i32, slow: i32 },
    #[error("rsi period must be positive, got {n}")]
    InvalidRsiPeriod { n: i32 },
    #[error("rsi low threshold must be less than high threshold, got high {high}, low {low}")]
    InvalidRsiThresholdOrder { high: i32, low: i32 },
    #[error("rsi threshold must be between 0 and 100, got {value}")]
    InvalidRsiThresholdRange { value: i32 },
    #[error("bears power period must be positive, got {n}")]
    InvalidBearsPeriod { n: i32 },
    #[error("bulls power period must be positive, got {n}")]
    InvalidBullsPeriod { n: i32 },
    #[error("chaikin oscillator fast period must be positive, got {n}")]
    InvalidChoFastPeriod { n: i32 },
    #[error("chaikin oscillator slow period must be positive, got {n}")]
    InvalidChoSlowPeriod { n: i32 },
    #[error(
        "chaikin oscillator fast period must not exceed slow period, got fast {fast}, slow {slow}"
    )]
    InvalidChoPeriodOrder { fast: i32, slow: i32 },
    #[error("chaikin volatility smoothing period must be positive, got {n}")]
    InvalidChvSmoothPeriod { n: i32 },
    #[error("chaikin volatility period must be positive, got {n}")]
    InvalidChvPeriod { n: i32 },
    #[error("dema period must be positive, got {n}")]
    InvalidDemaPeriod { n: i32 },
    #[error("detrended price oscillator period must be positive, got {n}")]
    InvalidDpoPeriod { n: i32 },
    #[error("force index period must be positive, got {n}")]
    InvalidForcePeriod { n: i32 },
    #[error("money flow index period must be positive, got {n}")]
    InvalidMfiPeriod { n: i32 },
    #[error(
        "money flow index low threshold must be less than high threshold, got high {high}, low {low}"
    )]
    InvalidMfiThresholdOrder { high: i32, low: i32 },
    #[error("money flow index threshold must be between 0 and 100, got {value}")]
    InvalidMfiThresholdRange { value: i32 },
    #[error("osma fast period must be positive, got {n}")]
    InvalidOsMaFastPeriod { n: i32 },
    #[error("osma slow period must be positive, got {n}")]
    InvalidOsMaSlowPeriod { n: i32 },
    #[error("osma signal period must be positive, got {n}")]
    InvalidOsMaSignalPeriod { n: i32 },
    #[error("osma fast period must not exceed slow period, got fast {fast}, slow {slow}")]
    InvalidOsMaPeriodOrder { fast: i32, slow: i32 },
    #[error("rate of change period must be positive, got {n}")]
    InvalidRocPeriod { n: i32 },
    #[error("relative vigor index period must be positive, got {n}")]
    InvalidRviPeriod { n: i32 },
    #[error("stochastic k period must be positive, got {n}")]
    InvalidStochasticKPeriod { n: i32 },
    #[error("stochastic slowing must be positive, got {n}")]
    InvalidStochasticSlowing { n: i32 },
    #[error(
        "stochastic low threshold must be less than high threshold, got high {high}, low {low}"
    )]
    InvalidStochasticThresholdOrder { high: i32, low: i32 },
    #[error("stochastic threshold must be between 0 and 100, got {value}")]
    InvalidStochasticThresholdRange { value: i32 },
    #[error("tema period must be positive, got {n}")]
    InvalidTemaPeriod { n: i32 },
    #[error("trix period must be positive, got {n}")]
    InvalidTrixPeriod { n: i32 },
    #[error("volume rate of change period must be positive, got {n}")]
    InvalidVrocPeriod { n: i32 },
    #[error("not enough data points to compute indicator: saw {points}, required {required}")]
    NotEnoughData { points: usize, required: usize },
}

// One flat arm per indicator: it grows with every indicator, and splitting it
// would only scatter the dispatch.
#[allow(clippy::too_many_lines)]
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
            let buffers = macd::compute(points, *fast_type, *fast_n, *slow_type, *slow_n)?;
            Ok(IndicatorOutput::Crossover {
                instrument,
                buffers,
            })
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
        IndicatorParameters::Dema { period } => {
            let buffers = dema::compute(points, *period)?;
            Ok(IndicatorOutput::Crossover {
                instrument,
                buffers,
            })
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
        IndicatorParameters::Mfi {
            period,
            high_threshold,
            low_threshold,
        } => {
            let buffer = mfi::compute(points, *period, *high_threshold, *low_threshold)?;
            Ok(IndicatorOutput::Threshold { instrument, buffer })
        }
        IndicatorParameters::Roc { period } => {
            let buffer = roc::compute(points, *period)?;
            Ok(IndicatorOutput::Oscillator { instrument, buffer })
        }
        IndicatorParameters::Rvi { period } => {
            let buffers = rvi::compute(points, *period)?;
            Ok(IndicatorOutput::Crossover {
                instrument,
                buffers,
            })
        }
        IndicatorParameters::Stochastic {
            k_period,
            slowing,
            high_threshold,
            low_threshold,
        } => {
            let buffer =
                stochastic::compute(points, *k_period, *slowing, *high_threshold, *low_threshold)?;
            Ok(IndicatorOutput::Threshold { instrument, buffer })
        }
        IndicatorParameters::Tema { period } => {
            let buffers = tema::compute(points, *period)?;
            Ok(IndicatorOutput::Crossover {
                instrument,
                buffers,
            })
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
    use crate::domain::indicator::{IndicatorParameters, MovingAverageType};
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
            IndicatorOutput::Threshold { buffer: buf, .. } => {
                assert_eq!(buf.high_threshold, 70.0);
                assert_eq!(buf.low_threshold, 30.0);
                assert!(!buf.items.is_empty());
            }
            other => panic!("expected OneThreshold, got {other:?}"),
        }
    }

    /// 14 points with constant close -> 10 MACD values (fast=3, slow=5,
    /// values start at index 4) and, after the 9-bar signal window, 2
    /// aligned crossover-buffer values (constant close -> 0.0).
    fn rp_macd_14() -> Vec<ResampledPointD1> {
        (1..=14)
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
    fn macd_dispatch_returns_crossover_buffers() {
        let params = IndicatorParameters::Macd {
            slow_type: MovingAverageType::Ema,
            slow_n: 5,
            fast_type: MovingAverageType::Ema,
            fast_n: 3,
        };
        let points = rp_macd_14();
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Crossover { buffers: bufs, .. } => {
                // 14 points -> 10 MACD values; the 9-bar signal window leaves
                // 2 aligned values (constant close -> 0.0).
                assert_eq!(bufs.upcross_long_buffer.len(), 2);
                assert_eq!(bufs.upcross_short_buffer.len(), 2);
                assert_eq!(bufs.upcross_long_buffer[0].value, 0.0);
                assert_eq!(bufs.upcross_short_buffer[0].value, 0.0);
                // Both buffers share the aligned timestamps (bar 13 and 14).
                assert_eq!(bufs.upcross_long_buffer[0].timestamp, 13);
                assert_eq!(bufs.upcross_short_buffer[0].timestamp, 13);
            }
            other => panic!("expected Crossover, got {other:?}"),
        }
    }

    #[test]
    fn macd_dispatch_with_insufficient_data_yields_empty_buffers() {
        let params = IndicatorParameters::Macd {
            slow_type: MovingAverageType::Ema,
            slow_n: 26,
            fast_type: MovingAverageType::Ema,
            fast_n: 12,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Crossover { buffers: bufs, .. } => {
                assert!(bufs.upcross_long_buffer.is_empty());
                assert!(bufs.upcross_short_buffer.is_empty());
            }
            other => panic!("expected Crossover, got {other:?}"),
        }
    }

    #[test]
    fn macd_invalid_fast_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Macd {
            slow_type: MovingAverageType::Ema,
            slow_n: 26,
            fast_type: MovingAverageType::Ema,
            fast_n: 0,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidMacdFastPeriod { n: 0 });
    }

    #[test]
    fn macd_invalid_slow_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Macd {
            slow_type: MovingAverageType::Ema,
            slow_n: 0,
            fast_type: MovingAverageType::Ema,
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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

    #[test]
    fn rvi_dispatch_returns_crossover_buffers() {
        let params = IndicatorParameters::Rvi { period: 2 };
        let points: Vec<ResampledPointD1> = (1..=8)
            .map(|i| ResampledPointD1 {
                id: i,
                instrument: "EURUSD".to_string(),
                open: 1.0,
                high: 2.0,
                low: 1.0,
                close: 2.0,
                volume: 1000,
                timestamp: i64::from(i),
            })
            .collect();
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Crossover { buffers, .. } => {
                // 8 points, period 2 -> 1 value at bar 8
                // (every bar closes at its high from a low open -> 1).
                assert_eq!(buffers.upcross_long_buffer.len(), 1);
                assert_eq!(buffers.upcross_long_buffer[0].timestamp, 8);
                assert_eq!(buffers.upcross_long_buffer[0].value, 1.0);
                assert_eq!(buffers.upcross_short_buffer[0].value, 1.0);
            }
            other => panic!("expected Crossover, got {other:?}"),
        }
    }

    #[test]
    fn rvi_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Rvi { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidRviPeriod { n: 0 });
    }

    #[test]
    fn dema_dispatch_returns_crossover_buffers() {
        let params = IndicatorParameters::Dema { period: 2 };
        let points = vec![
            rp(1, 1, 10.0),
            rp(2, 2, 10.0),
            rp(3, 3, 10.0),
            rp(4, 4, 10.0),
        ];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Crossover { buffers: bufs, .. } => {
                // 4 points, period 2 -> 2 values from bar 3 (constant close -> 10.0).
                assert_eq!(bufs.upcross_long_buffer.len(), 2);
                assert_eq!(bufs.upcross_short_buffer.len(), 2);
                assert_eq!(bufs.upcross_short_buffer[0].value, 10.0);
                assert_eq!(bufs.upcross_short_buffer[0].timestamp, 3);
            }
            other => panic!("expected Crossover, got {other:?}"),
        }
    }

    #[test]
    fn dema_dispatch_with_insufficient_data_yields_empty_buffers() {
        let params = IndicatorParameters::Dema { period: 14 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Crossover { buffers: bufs, .. } => {
                assert!(bufs.upcross_long_buffer.is_empty());
                assert!(bufs.upcross_short_buffer.is_empty());
            }
            other => panic!("expected Crossover, got {other:?}"),
        }
    }

    #[test]
    fn dema_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Dema { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidDemaPeriod { n: 0 });
    }

    #[test]
    fn tema_dispatch_returns_crossover_buffers() {
        let params = IndicatorParameters::Tema { period: 2 };
        let points = vec![
            rp(1, 1, 10.0),
            rp(2, 2, 10.0),
            rp(3, 3, 10.0),
            rp(4, 4, 10.0),
            rp(5, 5, 10.0),
        ];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Crossover { buffers: bufs, .. } => {
                // 5 points, period 2 -> 2 values from bar 4 (constant close -> 10.0).
                assert_eq!(bufs.upcross_long_buffer.len(), 2);
                assert_eq!(bufs.upcross_short_buffer.len(), 2);
                assert_eq!(bufs.upcross_short_buffer[0].value, 10.0);
                assert_eq!(bufs.upcross_short_buffer[0].timestamp, 4);
            }
            other => panic!("expected Crossover, got {other:?}"),
        }
    }

    #[test]
    fn tema_dispatch_with_insufficient_data_yields_empty_buffers() {
        let params = IndicatorParameters::Tema { period: 14 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Crossover { buffers: bufs, .. } => {
                assert!(bufs.upcross_long_buffer.is_empty());
                assert!(bufs.upcross_short_buffer.is_empty());
            }
            other => panic!("expected Crossover, got {other:?}"),
        }
    }

    #[test]
    fn tema_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Tema { period: 0 };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidTemaPeriod { n: 0 });
    }

    #[test]
    fn mfi_dispatch_returns_threshold_variant() {
        let params = IndicatorParameters::Mfi {
            period: 2,
            high_threshold: 80,
            low_threshold: 20,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0), rp(3, 3, 3.0), rp(4, 4, 4.0)];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Threshold { buffer: buf, .. } => {
                // 4 points, period 2 -> 2 values from bar 3 (rising price -> 100).
                assert_eq!(buf.high_threshold, 80.0);
                assert_eq!(buf.low_threshold, 20.0);
                assert_eq!(buf.items.len(), 2);
                assert_eq!(buf.items[0].timestamp, 3);
                assert_eq!(buf.items[0].value, 100.0);
            }
            other => panic!("expected Threshold, got {other:?}"),
        }
    }

    #[test]
    fn mfi_invalid_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Mfi {
            period: 0,
            high_threshold: 80,
            low_threshold: 20,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidMfiPeriod { n: 0 });
    }

    #[test]
    fn stochastic_dispatch_returns_threshold_variant() {
        let params = IndicatorParameters::Stochastic {
            k_period: 2,
            slowing: 2,
            high_threshold: 80,
            low_threshold: 20,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0), rp(3, 3, 3.0), rp(4, 4, 4.0)];
        let result = create_indicator_output(&params, &points).unwrap();
        match &result {
            IndicatorOutput::Threshold { buffer: buf, .. } => {
                // 4 points, k_period 2, slowing 2 -> 2 values from bar 3
                // (close at the highest high -> 100).
                assert_eq!(buf.high_threshold, 80.0);
                assert_eq!(buf.low_threshold, 20.0);
                assert_eq!(buf.items.len(), 2);
                assert_eq!(buf.items[0].timestamp, 3);
                assert_eq!(buf.items[0].value, 100.0);
            }
            other => panic!("expected Threshold, got {other:?}"),
        }
    }

    #[test]
    fn stochastic_invalid_k_period_propagates_error_through_dispatch() {
        let params = IndicatorParameters::Stochastic {
            k_period: 0,
            slowing: 3,
            high_threshold: 80,
            low_threshold: 20,
        };
        let points = vec![rp(1, 1, 1.0), rp(2, 2, 2.0)];
        let err = create_indicator_output(&params, &points).unwrap_err();
        assert_eq!(err, IndicatorError::InvalidStochasticKPeriod { n: 0 });
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => {
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
            IndicatorOutput::Oscillator { buffer: buf, .. } => assert!(buf.items.is_empty()),
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
                IndicatorError::InvalidDemaPeriod { n: 0 },
                "dema period must be positive, got 0",
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
                IndicatorError::InvalidMfiPeriod { n: 0 },
                "money flow index period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidMfiThresholdOrder { high: 20, low: 80 },
                "money flow index low threshold must be less than high threshold, got high 20, low 80",
            ),
            (
                IndicatorError::InvalidMfiThresholdRange { value: 101 },
                "money flow index threshold must be between 0 and 100, got 101",
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
                IndicatorError::InvalidRviPeriod { n: 0 },
                "relative vigor index period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidStochasticKPeriod { n: 0 },
                "stochastic k period must be positive, got 0",
            ),
            (
                IndicatorError::InvalidStochasticSlowing { n: 0 },
                "stochastic slowing must be positive, got 0",
            ),
            (
                IndicatorError::InvalidStochasticThresholdOrder { high: 20, low: 80 },
                "stochastic low threshold must be less than high threshold, got high 20, low 80",
            ),
            (
                IndicatorError::InvalidStochasticThresholdRange { value: 101 },
                "stochastic threshold must be between 0 and 100, got 101",
            ),
            (
                IndicatorError::InvalidTemaPeriod { n: 0 },
                "tema period must be positive, got 0",
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
