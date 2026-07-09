use crate::domain::indicator::{Indicator, IndicatorParameters};
use crate::domain::indicators::{generate_indicator_values, IndicatorError};
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::IndicatorValues;
use crate::domain::signal_group::SignalGroup;

/// Abstraction over the "parameters + points → `IndicatorValues`" step.
///
/// Placed at the application layer so that SignalGroup-level tests can
/// inject a made-up oscillator and verify the full path (injected compute →
/// `IndicatorValues` → `generate_signals` → `SignalGroup`) — including
/// crossing-timestamp correctness — without depending on real indicator math.
pub trait IndicatorComputer {
    fn compute(
        &self,
        parameters: &IndicatorParameters,
        points: &[ResampledPointD1],
    ) -> Result<IndicatorValues, IndicatorError>;
}

/// Default implementation that delegates to the domain dispatch
/// `generate_indicator_values`.
pub struct DefaultIndicatorComputer;

impl IndicatorComputer for DefaultIndicatorComputer {
    fn compute(
        &self,
        parameters: &IndicatorParameters,
        points: &[ResampledPointD1],
    ) -> Result<IndicatorValues, IndicatorError> {
        generate_indicator_values(parameters, points)
    }
}

/// Create a `SignalGroup` for each indicator using the injected `computer`
/// to produce indicator values, then deriving long/short signals from them.
///
/// Indicators whose computation fails (e.g. unsupported) are skipped silently
/// rather than aborting the run.
pub fn create_signals_with_computer(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    computer: &dyn IndicatorComputer,
) -> Vec<SignalGroup> {
    indicators
        .iter()
        .filter_map(|indicator| {
            let values: IndicatorValues = computer
                .compute(&indicator.parameters, resampled_points)
                .ok()?;
            Some(values.generate_signals(indicator.id))
        })
        .collect()
}

/// Create a `SignalGroup` for each indicator by computing its indicator
/// values from `resampled_points` and deriving long/short signals from them.
///
/// The `count` parameter is retained for signature stability with the
/// previous random-stub implementation but is **not used**: the number of
/// signals is now determined by the data, not requested. It is marked for
/// removal in a later cleanup.
///
/// Indicators whose computation is unsupported (e.g. MACD, out of scope for
/// this migration) are skipped silently rather than aborting the run.
///
/// Delegates to `create_signals_with_computer` with `DefaultIndicatorComputer`,
/// preserving the original signature and behavior (backward-compatible).
pub fn create_signals(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    _count: usize,
) -> Vec<SignalGroup> {
    create_signals_with_computer(indicators, resampled_points, &DefaultIndicatorComputer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::{indicators_rsi_single, resampled_points_for_rsi};
    use crate::domain::indicator::{Indicator, IndicatorParameters};
    use crate::domain::indicators::test_support;
    use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};
    use crate::domain::signal::{IndicatorValue, OneBuffer};

    #[test]
    fn rsi_indicator_produces_a_signal_group_stamped_with_its_id() {
        // 5 points (1,2,1,2,1), n=3 -> diffs [+1,-1,+1,-1], 2 RSI windows.
        // window [+1,-1,+1] -> rsi 67; window [-1,+1,-1] -> rsi 33.
        // Thresholds 70/30: 67 and 33 never cross 70 or 30 -> no signals.
        // So this verifies the path runs and returns a group with no signals.
        let indicators = indicators_rsi_single();
        let points = resampled_points_for_rsi();
        let result = create_signals(&indicators, &points, 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 1);
        // RSI 67/33 sit between 30 and 70 -> no threshold crossing -> no signals.
        assert!(result[0].long_signals.is_empty());
        assert!(result[0].short_signals.is_empty());
    }

    #[test]
    fn count_parameter_is_ignored_signal_count_comes_from_data() {
        let indicators = indicators_rsi_single();
        let points = resampled_points_for_rsi();
        let with_count_a = create_signals(&indicators, &points, 0);
        let with_count_b = create_signals(&indicators, &points, 1000);
        assert_eq!(with_count_a.len(), with_count_b.len());
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
        let result = create_signals(&indicators, &points, 100);
        // Only the RSI indicator yields a group.
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 2);
    }

    #[test]
    fn no_indicators_yields_no_signal_groups() {
        let points = resampled_points_for_rsi();
        let result = create_signals(&[], &points, 100);
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
        let result = create_signals(&indicators, &points, 100);
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
        let result = create_signals(&indicators, &points, 100);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 7);
        // RSI 0 -> 50 crosses above low threshold 30 -> long signal at ts 4000.
        assert_eq!(result[0].long_signals, vec![4000]);
        assert!(result[0].short_signals.is_empty());
    }

    // --- Level 1: SignalGroup via dependency injection ---

    /// A made-up oscillator that ignores parameters and returns a known
    /// alternating sequence `[-1, +1, -1, +1, -1]` stamped with the first 5
    /// resampled-point timestamps. Used to verify signal generation and
    /// timestamp routing independently of real indicator math.
    struct CrossingOscillator;

    impl IndicatorComputer for CrossingOscillator {
        fn compute(
            &self,
            _parameters: &IndicatorParameters,
            points: &[ResampledPointD1],
        ) -> Result<IndicatorValues, IndicatorError> {
            let items: Vec<IndicatorValue> = points
                .iter()
                .take(5)
                .enumerate()
                .map(|(i, p)| IndicatorValue {
                    timestamp: p.timestamp,
                    value: if i % 2 == 0 { -1.0 } else { 1.0 },
                })
                .collect();
            Ok(IndicatorValues::One(OneBuffer { items }))
        }
    }

    #[test]
    fn level1_injected_oscillator_produces_expected_crossing_timestamps() {
        // Uses real EURUSD resampled points from the development DB so the
        // timestamps are real. Skips gracefully if the DB is unavailable.
        let points = match test_support::fetch_eurusd_points() {
            Some(p) => p,
            None => {
                eprintln!("skipping level1_di_test: development DB unavailable");
                return;
            }
        };
        let ts: Vec<i64> = points.iter().take(5).map(|p| p.timestamp).collect();

        let indicators = vec![Indicator {
            id: 1,
            r#type: "awesome_oscillator".to_string(),
            parameters: IndicatorParameters::AwesomeOscillator,
            identifier: "ao_1".to_string(),
        }];

        let result = create_signals_with_computer(&indicators, &points, &CrossingOscillator);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 1);
        // Alternating [-1, +1, -1, +1, -1]:
        //   up-crosses (neg→pos) at index 1 and 3 -> long at ts[1], ts[3]
        //   down-crosses (pos→neg) at index 2 and 4 -> short at ts[2], ts[4]
        assert_eq!(result[0].long_signals, vec![ts[1], ts[3]]);
        assert_eq!(result[0].short_signals, vec![ts[2], ts[4]]);
    }

    #[test]
    fn default_computer_delegates_to_domain_dispatch() {
        // The DefaultIndicatorComputer must produce the same result as the
        // direct domain dispatch — verifying the delegation is correct.
        let indicators = indicators_rsi_single();
        let points = resampled_points_for_rsi();
        let via_default =
            create_signals_with_computer(&indicators, &points, &DefaultIndicatorComputer);
        let via_direct = create_signals(&indicators, &points, 100);
        assert_eq!(via_default, via_direct);
    }
}
