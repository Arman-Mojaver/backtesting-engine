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
    pub fn create_signal_group(self, indicator_id: i32) -> SignalGroup {
        let (instrument, (long_signals, short_signals)) = match self {
            Self::Oscillator { instrument, buffer } => (instrument, oscillator::create(&buffer)),
            Self::Threshold { instrument, buffer } => (instrument, thresholds::create(&buffer)),
            Self::Crossover { instrument, buffer } => (instrument, crossover::create(&buffer)),
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
                items: vec![iv(1, -1.0), iv(2, 1.0), iv(3, -1.0)],
            },
        };
        let result = values.create_signal_group(5);
        assert_eq!(
            result,
            SignalGroup {
                indicator_id: 5,
                long_signals: vec![2],
                short_signals: vec![3],
                operation_count: 2,
                instrument: "EURUSD".to_string(),
            }
        );
    }

    #[test]
    fn threshold_variant_routes_to_thresholds() {
        let values = IndicatorOutput::Threshold {
            instrument: "EURUSD".to_string(),
            buffer: ThresholdSeries {
                items: vec![iv(1, 20.0), iv(2, 40.0), iv(3, 80.0), iv(4, 60.0)],
                high_threshold: 70.0,
                low_threshold: 30.0,
            },
        };
        let result = values.create_signal_group(9);
        assert_eq!(
            result,
            SignalGroup {
                indicator_id: 9,
                long_signals: vec![2],
                short_signals: vec![4],
                operation_count: 2,
                instrument: "EURUSD".to_string(),
            }
        );
    }

    #[test]
    fn crossover_variant_routes_to_crossover() {
        let values = IndicatorOutput::Crossover {
            instrument: "EURUSD".to_string(),
            buffer: CrossoverPair {
                upcross_long_buffer: vec![iv(1, 0.0), iv(2, 2.0), iv(3, 0.0)],
                upcross_short_buffer: vec![iv(1, 1.0), iv(2, 1.0), iv(3, 1.0)],
            },
        };
        let result = values.create_signal_group(42);
        assert_eq!(
            result,
            SignalGroup {
                indicator_id: 42,
                long_signals: vec![2],
                short_signals: vec![3],
                operation_count: 2,
                instrument: "EURUSD".to_string(),
            }
        );
    }

    #[test]
    fn oscillator_empty_buffer_routes_to_empty() {
        let values = IndicatorOutput::Oscillator {
            instrument: "EURUSD".to_string(),
            buffer: OscillatorSeries { items: vec![] },
        };
        let result = values.create_signal_group(1);
        assert_eq!(
            result,
            SignalGroup {
                indicator_id: 1,
                long_signals: vec![],
                short_signals: vec![],
                operation_count: 0,
                instrument: "EURUSD".to_string(),
            }
        );
    }
}
