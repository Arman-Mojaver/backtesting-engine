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
pub struct OscillatorBuffer {
    pub items: Vec<IndicatorValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThresholdBuffer {
    pub items: Vec<IndicatorValue>,
    pub high_threshold: f64,
    pub low_threshold: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CrossoverBuffers {
    pub upcross_long_buffer: Vec<IndicatorValue>,
    pub upcross_short_buffer: Vec<IndicatorValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorOutput {
    Oscillator {
        instrument: String,
        buffer: OscillatorBuffer,
    },
    Threshold {
        instrument: String,
        buffer: ThresholdBuffer,
    },
    Crossover {
        instrument: String,
        buffers: CrossoverBuffers,
    },
}

impl IndicatorOutput {
    pub fn create_signal_group(self, indicator_id: i32) -> SignalGroup {
        let (instrument, (long_signals, short_signals)) = match self {
            Self::Oscillator { instrument, buffer } => {
                (instrument, oscillator::detect_signals(&buffer))
            }
            Self::Threshold { instrument, buffer } => {
                (instrument, thresholds::detect_signals(&buffer))
            }
            Self::Crossover {
                instrument,
                buffers,
            } => (instrument, crossover::detect_signals(&buffers)),
        };
        SignalGroup {
            indicator_id,
            long_signals,
            short_signals,
            instrument,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::iv;

    #[test]
    fn oscillator_variant_routes_to_oscillator() {
        let values = IndicatorOutput::Oscillator {
            instrument: "EURUSD".to_string(),
            buffer: OscillatorBuffer {
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
                instrument: "EURUSD".to_string(),
            }
        );
    }

    #[test]
    fn threshold_variant_routes_to_thresholds() {
        let values = IndicatorOutput::Threshold {
            instrument: "EURUSD".to_string(),
            buffer: ThresholdBuffer {
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
                instrument: "EURUSD".to_string(),
            }
        );
    }

    #[test]
    fn crossover_variant_routes_to_crossover() {
        let values = IndicatorOutput::Crossover {
            instrument: "EURUSD".to_string(),
            buffers: CrossoverBuffers {
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
                instrument: "EURUSD".to_string(),
            }
        );
    }

    #[test]
    fn oscillator_empty_buffer_routes_to_empty() {
        let values = IndicatorOutput::Oscillator {
            instrument: "EURUSD".to_string(),
            buffer: OscillatorBuffer { items: vec![] },
        };
        let result = values.create_signal_group(1);
        assert_eq!(
            result,
            SignalGroup {
                indicator_id: 1,
                long_signals: vec![],
                short_signals: vec![],
                instrument: "EURUSD".to_string(),
            }
        );
    }
}
