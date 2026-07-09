pub mod ac;
pub mod ao;
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
}
