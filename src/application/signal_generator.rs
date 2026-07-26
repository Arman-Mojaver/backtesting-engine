use rayon::prelude::*;

use crate::domain::indicator::Indicator;
use crate::domain::indicators::create_indicator_output;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::IndicatorOutput;
use crate::domain::signal_group::SignalGroup;

pub fn create_signal_groups(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
) -> Vec<SignalGroup> {
    indicators
        .par_iter()
        .filter_map(|indicator| create_signal_group(indicator, resampled_points))
        .collect()
}

fn create_signal_group(
    indicator: &Indicator,
    resampled_points: &[ResampledPointD1],
) -> Option<SignalGroup> {
    let indicator_output: IndicatorOutput =
        create_indicator_output(&indicator.parameters, resampled_points).ok()?;
    Some(indicator_output.create_signal_group(indicator.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::{indicators_rsi_single, resampled_points_for_rsi};
    use crate::domain::indicator::{Indicator, IndicatorParameters};
    use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};

    #[test]
    fn rsi_indicator_produces_a_signal_group_stamped_with_its_id() {
        // 5 points (1,2,1,2,1), n=3 -> diffs [+1,-1,+1,-1], 2 RSI windows.
        // window [+1,-1,+1] -> rsi 67; window [-1,+1,-1] -> rsi 33.
        // Thresholds 70/30: 67 and 33 never cross 70 or 30 -> no signals.
        // So this verifies the path runs and returns a group with no signals.
        let indicators = indicators_rsi_single();
        let points = resampled_points_for_rsi();
        let result = create_signal_groups(&indicators, &points);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 1);
        // RSI 67/33 sit between 30 and 70 -> no threshold crossing -> no signals.
        assert!(result[0].long_signals.is_empty());
        assert!(result[0].short_signals.is_empty());
    }

    #[test]
    fn unsupported_indicator_is_skipped_not_aborted() {
        // MACD is unsupported -> produces no group, but RSI still produces one.
        let indicators = vec![
            Indicator {
                id: 1,
                r#type: "macd".to_string(),
                parameters: IndicatorParameters::Macd {
                    slow_type: "ema".to_string(),
                    slow_n: 26,
                    fast_type: "ema".to_string(),
                    fast_n: 12,
                },
                identifier: "macd_1".to_string(),
            },
            Indicator {
                id: 2,
                r#type: "rsi".to_string(),
                parameters: IndicatorParameters::Rsi {
                    n: 3,
                    high_threshold: 70,
                    low_threshold: 30,
                },
                identifier: "rsi_2".to_string(),
            },
        ];
        let points = resampled_points_for_rsi();
        let result = create_signal_groups(&indicators, &points);
        // Only the RSI indicator yields a group.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 2);
    }

    #[test]
    fn no_indicators_yields_no_signal_groups() {
        let points = resampled_points_for_rsi();
        let result = create_signal_groups(&[], &points);
        assert!(result.is_empty());
    }

    #[test]
    fn all_unsupported_yields_no_signal_groups() {
        let indicators = vec![Indicator {
            id: 1,
            r#type: "macd".to_string(),
            parameters: IndicatorParameters::Macd {
                slow_type: "ema".to_string(),
                slow_n: 26,
                fast_type: "ema".to_string(),
                fast_n: 12,
            },
            identifier: "macd_1".to_string(),
        }];
        let points = resampled_points_for_rsi();
        let result = create_signal_groups(&indicators, &points);
        assert!(result.is_empty());
    }

    #[test]
    fn rsi_crossing_thresholds_produces_signals() {
        // Prices 5,4,3,4 with n=2: diffs [-1,-1,+1], RSI windows:
        //   window [-1,-1] -> gain 0, loss 2 -> rs 0 -> rsi 0 (ts 3000)
        //   window [-1,+1] -> gain 1, loss 1 -> rs 1 -> rsi 50 (ts 4000)
        // Thresholds 70/30: prev rsi 0 < low 30, curr rsi 50 > low 30 -> long.
        let points = vec![
            ResampledPointD1 {
                id: 1,
                instrument: "EURUSD".into(),
                open: 5.0,
                high: 5.1,
                low: 4.9,
                close: 5.0,
                volume: 1000,
                timestamp: 1000,
                high_low_order: HighLowOrder::HighFirst,
            },
            ResampledPointD1 {
                id: 2,
                instrument: "EURUSD".into(),
                open: 4.0,
                high: 4.1,
                low: 3.9,
                close: 4.0,
                volume: 1000,
                timestamp: 2000,
                high_low_order: HighLowOrder::HighFirst,
            },
            ResampledPointD1 {
                id: 3,
                instrument: "EURUSD".into(),
                open: 3.0,
                high: 3.1,
                low: 2.9,
                close: 3.0,
                volume: 1000,
                timestamp: 3000,
                high_low_order: HighLowOrder::HighFirst,
            },
            ResampledPointD1 {
                id: 4,
                instrument: "EURUSD".into(),
                open: 4.0,
                high: 4.1,
                low: 3.9,
                close: 4.0,
                volume: 1000,
                timestamp: 4000,
                high_low_order: HighLowOrder::HighFirst,
            },
        ];
        let indicators = vec![Indicator {
            id: 7,
            r#type: "rsi".to_string(),
            parameters: IndicatorParameters::Rsi {
                n: 2,
                high_threshold: 70,
                low_threshold: 30,
            },
            identifier: "rsi_2-70-30".to_string(),
        }];
        let result = create_signal_groups(&indicators, &points);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 7);
        // RSI 0 -> 50 crosses above low threshold 30 -> long signal at ts 4000.
        assert_eq!(result[0].long_signals, vec![4000]);
        assert!(result[0].short_signals.is_empty());
    }
}
