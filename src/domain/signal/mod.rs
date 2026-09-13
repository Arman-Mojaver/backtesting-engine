pub mod crossover;
pub mod oscillator;
pub mod thresholds;

use crate::domain::signal_group::SignalGroup;

#[derive(Debug, Clone, PartialEq)]
pub struct IndicatorValue {
    pub timestamp: i64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OscillatorSeries {
    pub items: Vec<IndicatorValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdSeries {
    pub items: Vec<IndicatorValue>,
    pub high_threshold: f64,
    pub low_threshold: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrossoverPair {
    pub upcross_long_buffer: Vec<IndicatorValue>,
    pub upcross_short_buffer: Vec<IndicatorValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorOutput {
    Oscillator {
        instrument: String,
        buffer: OscillatorSeries,
    },
    Threshold {
        instrument: String,
        buffer: ThresholdSeries,
    },
    Crossover {
        instrument: String,
        buffer: CrossoverPair,
    },
}

impl IndicatorOutput {
    pub fn create_signal_group(&self, indicator_id: i32) -> SignalGroup {
        let (long_signals, short_signals, instrument) = match self {
            Self::Oscillator {
                instrument,
                buffer: series,
            } => {
                let (long, short) = oscillator::create(series);
                (long, short, instrument.clone())
            }
            Self::Threshold {
                instrument,
                buffer: series,
            } => {
                let (long, short) = thresholds::create(series);
                (long, short, instrument.clone())
            }
            Self::Crossover {
                instrument,
                buffer: pair,
            } => {
                let (long, short) = crossover::create(pair);
                (long, short, instrument.clone())
            }
        };
        let operation_count = long_signals.len().saturating_add(short_signals.len());
        SignalGroup {
            indicator_id,
            long_signals,
            short_signals,
            operation_count,
            instrument,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iv(timestamp: i64, value: f64) -> IndicatorValue {
        IndicatorValue { timestamp, value }
    }

    #[test]
    fn oscillator_variant_routes_to_oscillator() {
        let values = IndicatorOutput::Oscillator {
            instrument: "EURUSD".to_string(),
            buffer: OscillatorSeries {
                items: vec![iv(1, -1.0), iv(2, 1.0)],
            },
        };
        let result = values.create_signal_group(5);
        assert_eq!(result.indicator_id, 5);
        assert_eq!(result.long_signals, vec![2]);
        assert!(result.short_signals.is_empty());
        assert_eq!(result.instrument, "EURUSD");
    }

    #[test]
    fn threshold_variant_routes_to_thresholds() {
        let values = IndicatorOutput::Threshold {
            instrument: "EURUSD".to_string(),
            buffer: ThresholdSeries {
                items: vec![iv(1, -2.0), iv(2, 0.0)],
                high_threshold: 2.0,
                low_threshold: -1.0,
            },
        };
        let result = values.create_signal_group(9);
        assert_eq!(result.indicator_id, 9);
        assert_eq!(result.long_signals, vec![2]);
    }

    #[test]
    fn crossover_variant_routes_to_crossover() {
        let values = IndicatorOutput::Crossover {
            instrument: "EURUSD".to_string(),
            buffer: CrossoverPair {
                upcross_long_buffer: vec![iv(1, 0.0), iv(2, 2.0)],
                upcross_short_buffer: vec![iv(1, 1.0), iv(2, 1.0)],
            },
        };
        let result = values.create_signal_group(42);
        assert_eq!(result.indicator_id, 42);
        assert_eq!(result.long_signals, vec![2]);
    }

    #[test]
    fn oscillator_empty_buffer_routes_to_empty() {
        let values = IndicatorOutput::Oscillator {
            instrument: "EURUSD".to_string(),
            buffer: OscillatorSeries { items: vec![] },
        };
        let result = values.create_signal_group(1);
        assert!(result.long_signals.is_empty());
        assert!(result.short_signals.is_empty());
        assert_eq!(result.operation_count, 0);
    }
}
