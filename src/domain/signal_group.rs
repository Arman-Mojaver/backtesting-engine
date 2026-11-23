use crate::domain::annual_operation_count::annual_operation_count;
use crate::domain::strategy::GlobalStrategyFilter;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignalGroup {
    pub indicator_id: i32,
    pub long_signals: Vec<i64>,
    pub short_signals: Vec<i64>,
    /// Intentional: read only to fill `Strategy::instrument`, which is kept for
    /// tracing. The signal cache keys on the resampled points, not on this field.
    pub instrument: String,
}

impl SignalGroup {
    pub const fn signal_count(&self) -> usize {
        self.long_signals
            .len()
            .saturating_add(self.short_signals.len())
    }
}

pub fn filter_signal_groups_by_annual_ops(
    signal_groups: Vec<SignalGroup>,
    filter: &GlobalStrategyFilter,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Vec<SignalGroup> {
    signal_groups
        .into_iter()
        .filter(|signal_group| {
            // Intentional: skips annual_operation_count for empty groups (performance)
            signal_group.signal_count() > 0
                && filter.admits_activity(annual_operation_count(
                    signal_group.signal_count(),
                    start_timestamp,
                    end_timestamp,
                ))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::fixtures::ONE_YEAR_SECS;

    fn min_ops(min_annual_operation_count: f64) -> GlobalStrategyFilter {
        GlobalStrategyFilter {
            min_annual_operation_count,
            ..GlobalStrategyFilter::default()
        }
    }

    fn make_group(long_count: usize, short_count: usize) -> SignalGroup {
        SignalGroup {
            indicator_id: 1,
            long_signals: vec![0; long_count],
            short_signals: vec![0; short_count],
            instrument: "EURUSD".to_string(),
        }
    }

    #[test]
    fn both_empty_returns_zero() {
        let group = make_group(0, 0);
        assert_eq!(group.signal_count(), 0);
    }

    #[test]
    fn only_longs_returns_long_count() {
        let group = make_group(5, 0);
        assert_eq!(group.signal_count(), 5);
    }

    #[test]
    fn only_shorts_returns_short_count() {
        let group = make_group(0, 3);
        assert_eq!(group.signal_count(), 3);
    }

    #[test]
    fn both_populated_returns_sum() {
        let group = make_group(4, 7);
        assert_eq!(group.signal_count(), 11);
    }

    #[test]
    fn filter_keeps_group_meeting_threshold() {
        let group = make_group(10, 0);
        let result =
            filter_signal_groups_by_annual_ops(vec![group], &min_ops(10.0), 0, ONE_YEAR_SECS);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_excludes_group_below_threshold() {
        let group = make_group(5, 0);
        let result =
            filter_signal_groups_by_annual_ops(vec![group], &min_ops(10.0), 0, ONE_YEAR_SECS);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_excludes_empty_group() {
        let group = make_group(0, 0);
        let result =
            filter_signal_groups_by_annual_ops(vec![group], &min_ops(1.0), 0, ONE_YEAR_SECS);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_mixed_bag() {
        let groups = vec![
            SignalGroup {
                indicator_id: 1,
                ..make_group(20, 0)
            },
            SignalGroup {
                indicator_id: 2,
                ..make_group(3, 0)
            },
            SignalGroup {
                indicator_id: 3,
                ..make_group(0, 0)
            },
            SignalGroup {
                indicator_id: 4,
                ..make_group(12, 0)
            },
        ];
        let result = filter_signal_groups_by_annual_ops(groups, &min_ops(10.0), 0, ONE_YEAR_SECS);
        let kept_ids: Vec<i32> = result.iter().map(|group| group.indicator_id).collect();
        assert_eq!(kept_ids, vec![1, 4]);
    }

    #[test]
    fn filter_preserves_indicator_id() {
        let group = SignalGroup {
            indicator_id: 42,
            ..make_group(15, 0)
        };
        let result =
            filter_signal_groups_by_annual_ops(vec![group], &min_ops(10.0), 0, ONE_YEAR_SECS);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 42);
    }

    #[test]
    fn filter_empty_input_returns_empty() {
        let result = filter_signal_groups_by_annual_ops(vec![], &min_ops(10.0), 0, ONE_YEAR_SECS);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_counts_short_signals_toward_threshold() {
        let group = make_group(0, 10);
        let result =
            filter_signal_groups_by_annual_ops(vec![group], &min_ops(10.0), 0, ONE_YEAR_SECS);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_keeps_group_whose_combined_signals_meet_threshold() {
        let group = make_group(6, 4);
        let result =
            filter_signal_groups_by_annual_ops(vec![group], &min_ops(10.0), 0, ONE_YEAR_SECS);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_excludes_all_groups_when_range_is_empty() {
        let groups = vec![make_group(20, 0), make_group(0, 20)];
        let result = filter_signal_groups_by_annual_ops(groups, &min_ops(10.0), 1000, 1000);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_excludes_group_below_threshold_over_two_years() {
        let group = make_group(15, 0);
        let result =
            filter_signal_groups_by_annual_ops(vec![group], &min_ops(10.0), 0, 2 * ONE_YEAR_SECS);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_threshold_zero_keeps_non_empty() {
        let group = make_group(1, 0);
        let result =
            filter_signal_groups_by_annual_ops(vec![group], &min_ops(0.0), 0, ONE_YEAR_SECS);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_threshold_zero_excludes_empty() {
        let group = make_group(0, 0);
        let result =
            filter_signal_groups_by_annual_ops(vec![group], &min_ops(0.0), 0, ONE_YEAR_SECS);
        assert!(result.is_empty());
    }
}
