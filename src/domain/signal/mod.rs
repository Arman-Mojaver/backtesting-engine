pub mod crossover;
pub mod oscillator;
pub mod thresholds;

use serde::{Deserialize, Serialize};

use crate::domain::signal_group::SignalGroup;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorValue {
    pub timestamp: i64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OscillatorSeries {
    pub items: Vec<IndicatorValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ThresholdSeries {
    pub items: Vec<IndicatorValue>,
    pub high_threshold: f64,
    pub low_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CrossoverPair {
    pub upcross_long_buffer: Vec<IndicatorValue>,
    pub upcross_short_buffer: Vec<IndicatorValue>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndicatorOutput {
    Oscillator(OscillatorSeries),
    Threshold(ThresholdSeries),
    Crossover(CrossoverPair),
}

impl IndicatorOutput {
    pub fn create_signal_group(&self, indicator_id: i32) -> SignalGroup {
        match self {
            IndicatorOutput::Oscillator(buffer) => oscillator::create(buffer, indicator_id),
            IndicatorOutput::Threshold(buffer) => thresholds::create(buffer, indicator_id),
            IndicatorOutput::Crossover(buffers) => crossover::create(buffers, indicator_id),
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
    fn one_variant_routes_to_oscillator() {
        // up-cross through zero -> long signal at timestamp 2.
        let values = IndicatorOutput::Oscillator(OscillatorSeries {
            items: vec![iv(1, -1.0), iv(2, 1.0)],
        });
        let result = values.create_signal_group(5);
        assert_eq!(result.indicator_id, 5);
        assert_eq!(result.long_signals, vec![2]);
        assert!(result.short_signals.is_empty());
    }

    #[test]
    fn one_threshold_variant_routes_to_thresholds() {
        // cross above low threshold -> long at timestamp 2.
        let values = IndicatorOutput::Threshold(ThresholdSeries {
            items: vec![iv(1, -2.0), iv(2, 0.0)],
            high_threshold: 2.0,
            low_threshold: -1.0,
        });
        let result = values.create_signal_group(9);
        assert_eq!(result.indicator_id, 9);
        assert_eq!(result.long_signals, vec![2]);
    }

    #[test]
    fn two_variant_routes_to_crossover() {
        // long overtakes short -> up-cross -> long at timestamp 2.
        let values = IndicatorOutput::Crossover(CrossoverPair {
            upcross_long_buffer: vec![iv(1, 0.0), iv(2, 2.0)],
            upcross_short_buffer: vec![iv(1, 1.0), iv(2, 1.0)],
        });
        let result = values.create_signal_group(42);
        assert_eq!(result.indicator_id, 42);
        assert_eq!(result.long_signals, vec![2]);
    }

    #[test]
    fn one_empty_buffer_routes_to_empty_oscillator() {
        let values = IndicatorOutput::Oscillator(OscillatorSeries { items: vec![] });
        let result = values.create_signal_group(1);
        assert!(result.long_signals.is_empty());
        assert!(result.short_signals.is_empty());
    }
}
