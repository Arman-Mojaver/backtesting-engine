pub mod crossover;
pub mod oscillator;
pub mod thresholds;

use crate::domain::resampled_point::ResampledPointD1;
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
    pub fn create_signal_group(
        self,
        indicator_id: i32,
        resampled_points: &[ResampledPointD1],
    ) -> SignalGroup {
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
            long_signals: shift_to_next_resampled_point(&long_signals, resampled_points),
            short_signals: shift_to_next_resampled_point(&short_signals, resampled_points),
            instrument,
        }
    }
}

/// An indicator value at resampled point T needs T's close, while the
/// operation point stamped T is entered at T's open. The earliest operation
/// point a signal can select is therefore the next resampled point.
fn shift_to_next_resampled_point(
    signal_timestamps: &[i64],
    resampled_points: &[ResampledPointD1],
) -> Vec<i64> {
    signal_timestamps
        .iter()
        .filter_map(|&timestamp| next_resampled_point_timestamp(timestamp, resampled_points))
        .collect()
}

fn next_resampled_point_timestamp(
    timestamp: i64,
    resampled_points: &[ResampledPointD1],
) -> Option<i64> {
    let next_index = resampled_points.partition_point(|point| point.timestamp <= timestamp);
    resampled_points
        .get(next_index)
        .map(|point| point.timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::{iv, resampled_points_at};

    #[test]
    fn oscillator_variant_routes_to_oscillator() {
        let values = IndicatorOutput::Oscillator {
            instrument: "EURUSD".to_string(),
            buffer: OscillatorBuffer {
                items: vec![iv(1, -1.0), iv(2, 1.0), iv(3, -1.0)],
            },
        };
        let result = values.create_signal_group(5, &resampled_points_at(&[1, 2, 3, 4, 5]));
        assert_eq!(
            result,
            SignalGroup {
                indicator_id: 5,
                long_signals: vec![3],
                short_signals: vec![4],
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
        let result = values.create_signal_group(9, &resampled_points_at(&[1, 2, 3, 4, 5]));
        assert_eq!(
            result,
            SignalGroup {
                indicator_id: 9,
                long_signals: vec![3],
                short_signals: vec![5],
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
        let result = values.create_signal_group(42, &resampled_points_at(&[1, 2, 3, 4, 5]));
        assert_eq!(
            result,
            SignalGroup {
                indicator_id: 42,
                long_signals: vec![3],
                short_signals: vec![4],
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
        let result = values.create_signal_group(1, &resampled_points_at(&[1, 2, 3, 4, 5]));
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

    #[test]
    fn signal_on_a_resampled_point_enters_the_operation_point_of_the_next_resampled_point() {
        // The cross is read at the close of 200; the next resampled point is
        // 500 (a gap, e.g. a weekend), not 200 plus one day.
        let values = IndicatorOutput::Oscillator {
            instrument: "EURUSD".to_string(),
            buffer: OscillatorBuffer {
                items: vec![iv(100, -1.0), iv(200, 1.0), iv(500, -1.0), iv(600, -2.0)],
            },
        };
        let result = values.create_signal_group(1, &resampled_points_at(&[100, 200, 500, 600]));
        assert_eq!(result.long_signals, vec![500]);
        assert_eq!(result.short_signals, vec![600]);
    }

    #[test]
    fn signal_on_the_last_resampled_point_is_dropped() {
        let values = IndicatorOutput::Oscillator {
            instrument: "EURUSD".to_string(),
            buffer: OscillatorBuffer {
                items: vec![iv(1, -1.0), iv(2, 1.0)],
            },
        };
        let result = values.create_signal_group(1, &resampled_points_at(&[1, 2]));
        assert!(result.long_signals.is_empty());
        assert!(result.short_signals.is_empty());
    }
}
