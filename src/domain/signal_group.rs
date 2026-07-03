use serde::{Deserialize, Serialize};

use crate::domain::annual_operation_count::annual_operation_count;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalGroup {
    pub indicator_id: i32,
    pub long_signals: Vec<i64>,
    pub short_signals: Vec<i64>,
}

impl SignalGroup {
    pub fn operation_count(&self) -> usize {
        self.long_signals.len() + self.short_signals.len()
    }
}

pub fn filter_signal_groups_by_annual_ops(
    signal_groups: Vec<SignalGroup>,
    min_operation_count: f64,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Vec<SignalGroup> {
    signal_groups
        .into_iter()
        .filter(|sg| {
            annual_operation_count(sg.operation_count(), start_timestamp, end_timestamp)
                >= min_operation_count
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_YEAR_SECS: i64 = 365 * 86400;

    fn make_group(long_count: usize, short_count: usize) -> SignalGroup {
        SignalGroup {
            indicator_id: 1,
            long_signals: vec![0; long_count],
            short_signals: vec![0; short_count],
        }
    }

    fn make_group_with_timestamps(longs: Vec<i64>, shorts: Vec<i64>) -> SignalGroup {
        SignalGroup {
            indicator_id: 1,
            long_signals: longs,
            short_signals: shorts,
        }
    }

    // --- operation_count tests ---

    #[test]
    fn both_empty_returns_zero() {
        let group = make_group(0, 0);
        assert_eq!(group.operation_count(), 0);
    }

    #[test]
    fn only_longs_returns_long_count() {
        let group = make_group(5, 0);
        assert_eq!(group.operation_count(), 5);
    }

    #[test]
    fn only_shorts_returns_short_count() {
        let group = make_group(0, 3);
        assert_eq!(group.operation_count(), 3);
    }

    #[test]
    fn both_populated_returns_sum() {
        let group = make_group(4, 7);
        assert_eq!(group.operation_count(), 11);
    }

    // --- filter_signal_groups_by_annual_ops tests ---

    #[test]
    fn filter_keeps_group_meeting_threshold() {
        let group = make_group(10, 0);
        let result = filter_signal_groups_by_annual_ops(vec![group], 10.0, 0, ONE_YEAR_SECS);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_excludes_group_below_threshold() {
        let group = make_group(5, 0);
        let result = filter_signal_groups_by_annual_ops(vec![group], 10.0, 0, ONE_YEAR_SECS);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_excludes_empty_group() {
        let group = make_group(0, 0);
        let result = filter_signal_groups_by_annual_ops(vec![group], 1.0, 0, ONE_YEAR_SECS);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_mixed_bag() {
        let groups = vec![
            make_group(20, 0), // annual ≈ 20 → passes
            make_group(3, 0),  // annual ≈ 3  → fails
            make_group(0, 0),  // empty → fails
        ];
        let result = filter_signal_groups_by_annual_ops(groups, 10.0, 0, ONE_YEAR_SECS);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_preserves_indicator_id() {
        let group = SignalGroup {
            indicator_id: 42,
            long_signals: vec![0; 15],
            short_signals: vec![],
        };
        let result = filter_signal_groups_by_annual_ops(vec![group], 10.0, 0, ONE_YEAR_SECS);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 42);
    }

    #[test]
    fn filter_empty_input_returns_empty() {
        let result = filter_signal_groups_by_annual_ops(vec![], 10.0, 0, ONE_YEAR_SECS);
        assert!(result.is_empty());
    }

    #[test]
    fn filter_threshold_zero_keeps_non_empty() {
        let group = make_group(1, 0);
        let result = filter_signal_groups_by_annual_ops(vec![group], 0.0, 0, ONE_YEAR_SECS);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn filter_threshold_zero_excludes_empty() {
        let group = make_group(0, 0);
        let result = filter_signal_groups_by_annual_ops(vec![group], 0.0, 0, ONE_YEAR_SECS);
        assert!(result.is_empty());
    }
}
