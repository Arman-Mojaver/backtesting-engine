pub mod crossover;
pub mod oscillator;
pub mod thresholds;

use serde::{Deserialize, Serialize};

use crate::domain::signal_group::SignalGroup;

/// A single indicator value tagged with the resampled-point timestamp it describes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IndicatorValue {
    pub timestamp: i64,
    pub value: f64,
}

/// One buffer of indicator values (oscillator-style: signals on zero-cross).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OneBuffer {
    pub items: Vec<IndicatorValue>,
}

/// One buffer plus an upper/lower threshold band (threshold-style signals).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OneBufferThreshold {
    pub items: Vec<IndicatorValue>,
    pub high_threshold: f64,
    pub low_threshold: f64,
}

/// Two buffers to be compared by subtraction (crossover-style signals).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TwoBuffers {
    pub upcross_long_buffer: Vec<IndicatorValue>,
    pub upcross_short_buffer: Vec<IndicatorValue>,
}

/// The shape of indicator output determines which signal rule applies.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndicatorValues {
    One(OneBuffer),
    OneThreshold(OneBufferThreshold),
    Two(TwoBuffers),
}

impl IndicatorValues {
    /// Generate the long/short signal group for this indicator shape, stamped
    /// with `indicator_id`.
    pub fn generate_signals(&self, indicator_id: i32) -> SignalGroup {
        match self {
            IndicatorValues::One(buffer) => oscillator::generate(buffer, indicator_id),
            IndicatorValues::OneThreshold(buffer) => thresholds::generate(buffer, indicator_id),
            IndicatorValues::Two(buffers) => crossover::generate(buffers, indicator_id),
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
        let values = IndicatorValues::One(OneBuffer {
            items: vec![iv(1, -1.0), iv(2, 1.0)],
        });
        let result = values.generate_signals(5);
        assert_eq!(result.indicator_id, 5);
        assert_eq!(result.long_signals, vec![2]);
        assert!(result.short_signals.is_empty());
    }

    #[test]
    fn one_threshold_variant_routes_to_thresholds() {
        // cross above low threshold -> long at timestamp 2.
        let values = IndicatorValues::OneThreshold(OneBufferThreshold {
            items: vec![iv(1, -2.0), iv(2, 0.0)],
            high_threshold: 2.0,
            low_threshold: -1.0,
        });
        let result = values.generate_signals(9);
        assert_eq!(result.indicator_id, 9);
        assert_eq!(result.long_signals, vec![2]);
    }

    #[test]
    fn two_variant_routes_to_crossover() {
        // long overtakes short -> up-cross -> long at timestamp 2.
        let values = IndicatorValues::Two(TwoBuffers {
            upcross_long_buffer: vec![iv(1, 0.0), iv(2, 2.0)],
            upcross_short_buffer: vec![iv(1, 1.0), iv(2, 1.0)],
        });
        let result = values.generate_signals(42);
        assert_eq!(result.indicator_id, 42);
        assert_eq!(result.long_signals, vec![2]);
    }

    #[test]
    fn one_empty_buffer_routes_to_empty_oscillator() {
        let values = IndicatorValues::One(OneBuffer { items: vec![] });
        let result = values.generate_signals(1);
        assert!(result.long_signals.is_empty());
        assert!(result.short_signals.is_empty());
    }
}
