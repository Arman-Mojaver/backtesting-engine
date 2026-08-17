use crate::config::environment::{parquet_root_dir, Environment};
use anyhow::Context;
use std::collections::HashSet;

use crate::application::signal_generator::create_signal_groups;
use crate::application::strategy_data::{fetch_strategy_data, StrategyQuery};
use crate::application::strategy_generator::create_strategies;
use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoints;
use crate::domain::signal_group::{filter_signal_groups_by_annual_ops, SignalGroup};
use crate::domain::strategy::{filter_strategies, GlobalStrategyFilter, Strategy};
use crate::domain::validation::combined_validation::{validate_all, CombinedValidationError};
use crate::infrastructure::processed_manifest::ParquetProcessedManifest;
use crate::infrastructure::redis::{read_signal_groups, signal_groups_key, write_signal_groups};

pub fn get_unprocessed(all_indicator_ids: &[i32], processed_ids: &HashSet<i32>) -> Vec<i32> {
    all_indicator_ids
        .iter()
        .copied()
        .filter(|id| !processed_ids.contains(id))
        .collect()
}

pub fn diff_cached_signal_group_ids(cached: &[SignalGroup], needed: &[i32]) -> Vec<i32> {
    let cached_ids: HashSet<i32> = cached.iter().map(|sg| sg.indicator_id).collect();
    needed
        .iter()
        .copied()
        .filter(|id| !cached_ids.contains(id))
        .collect()
}

pub fn merge_signal_groups(
    mut cached: Vec<SignalGroup>,
    new: Vec<SignalGroup>,
) -> Vec<SignalGroup> {
    let cached_ids: HashSet<i32> = cached.iter().map(|sg| sg.indicator_id).collect();
    cached.extend(
        new.into_iter()
            .filter(|sg| !cached_ids.contains(&sg.indicator_id)),
    );
    cached
}

pub fn run_strategies(
    instrument: &str,
    mms_id: i32,
) -> anyhow::Result<Option<(Vec<Strategy>, Vec<i32>)>> {
    let env: Environment = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .parse()
        .unwrap_or(Environment::Development);

    // Step 1: Fetch shared data
    eprintln!(
        "Fetching shared data for {} (mms_id={}) ...",
        instrument, mms_id
    );
    let query = StrategyQuery {
        money_management_strategy_id: mms_id,
        instrument: instrument.to_string(),
    };
    let data = fetch_strategy_data(&query)?;

    validate_all(
        &data.long_operation_points,
        &data.short_operation_points,
        &data.indicators,
        &data.resampled_points,
        mms_id,
    )
    .map_err(|e: CombinedValidationError| anyhow::anyhow!("Validation failed: {:?}", e))?;

    let operation_points = OperationPoints {
        long_operation_points: data.long_operation_points,
        short_operation_points: data.short_operation_points,
    };
    let (start_timestamp, end_timestamp) = operation_points.start_end_timestamps();

    let root_dir = parquet_root_dir(env);
    let manifest = ParquetProcessedManifest::new(root_dir);
    let processed_rows = manifest.read(instrument, mms_id)?;
    let processed_ids: HashSet<i32> = processed_rows.iter().map(|r| r.indicator_id).collect();

    let all_indicator_ids: Vec<i32> = data.indicators.iter().map(|i| i.id).collect();

    let unprocessed_ids = get_unprocessed(&all_indicator_ids, &processed_ids);
    if unprocessed_ids.is_empty() {
        eprintln!(
            "All {} indicators already processed for {} (mms_id={}). Nothing to do.",
            all_indicator_ids.len(),
            instrument,
            mms_id,
        );
        return Ok(None);
    }

    eprintln!(
        "{} indicators total, {} unprocessed for {} (mms_id={})",
        all_indicator_ids.len(),
        unprocessed_ids.len(),
        instrument,
        mms_id,
    );

    let redis_key = signal_groups_key(instrument);
    let redis_client =
        redis::Client::open("redis://redis:6379/0").context("Failed to connect to Redis")?;
    let mut redis_conn = redis_client
        .get_connection()
        .context("Failed to get Redis connection")?;

    let cached_groups: Vec<SignalGroup> = read_signal_groups(&mut redis_conn, &redis_key)?
        .unwrap_or_else(|| {
            eprintln!("Redis cache MISS for key '{}'", redis_key);
            Vec::new()
        });

    if !cached_groups.is_empty() {
        eprintln!(
            "Redis cache HIT for key '{}': {} cached signal groups",
            redis_key,
            cached_groups.len()
        );
    }

    let indicators_to_compute = diff_cached_signal_group_ids(&cached_groups, &unprocessed_ids);

    let all_cached_groups = if indicators_to_compute.is_empty() {
        eprintln!("All unprocessed indicators already have cached signal groups.");
        cached_groups
    } else {
        eprintln!(
            "Computing signal groups for {} indicators ({} already cached) ...",
            indicators_to_compute.len(),
            cached_groups.len(),
        );

        let compute_set: HashSet<i32> = indicators_to_compute.iter().copied().collect();
        let indicators_for_compute: Vec<Indicator> = data
            .indicators
            .into_iter()
            .filter(|indicator| compute_set.contains(&indicator.id))
            .collect();

        let new_groups = create_signal_groups(&indicators_for_compute, &data.resampled_points);
        eprintln!(
            "Computed {} new signal groups (total operations: {})",
            new_groups.len(),
            new_groups
                .iter()
                .map(|sg| sg.operation_count)
                .sum::<usize>(),
        );

        let merged = merge_signal_groups(cached_groups, new_groups);
        eprintln!(
            "Writing {} signal groups to Redis key '{}' ...",
            merged.len(),
            redis_key
        );
        write_signal_groups(&mut redis_conn, &redis_key, &merged)?;
        merged
    };

    let unprocessed_set: HashSet<i32> = unprocessed_ids.iter().copied().collect();
    let all_cached_groups: Vec<SignalGroup> = all_cached_groups
        .into_iter()
        .filter(|sg| unprocessed_set.contains(&sg.indicator_id))
        .collect();

    let global_filter = GlobalStrategyFilter::default();
    let filtered_groups = filter_signal_groups_by_annual_ops(
        all_cached_groups,
        global_filter.min_operation_count,
        start_timestamp,
        end_timestamp,
    );
    eprintln!(
        "After annual operation count filter (min {}): {} signal groups",
        global_filter.min_operation_count,
        filtered_groups.len(),
    );

    eprintln!("Creating strategies ...");
    let strategies = create_strategies(
        &filtered_groups,
        &operation_points,
        mms_id,
        start_timestamp,
        end_timestamp,
    );

    let strategies = filter_strategies(strategies, &global_filter);
    eprintln!(
        "After strategy filter (min annual ROI {}, max drawdown {}): {} strategies",
        global_filter.min_annual_roi,
        global_filter.max_draw_down_max,
        strategies.len(),
    );

    Ok(Some((strategies, unprocessed_ids)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::signal_group::SignalGroup;

    // --- get_unprocessed tests ---

    #[test]
    fn diff_processed_empty_both_returns_empty() {
        let result = get_unprocessed(&[], &HashSet::new());
        assert!(result.is_empty());
    }

    #[test]
    fn diff_processed_empty_processed_returns_all() {
        let result = get_unprocessed(&[1, 2, 3], &HashSet::new());
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn diff_processed_full_overlap_returns_empty() {
        let mut processed = HashSet::new();
        processed.insert(1);
        processed.insert(2);
        let result = get_unprocessed(&[1, 2], &processed);
        assert!(result.is_empty());
    }

    #[test]
    fn diff_processed_partial_overlap_returns_unprocessed() {
        let mut processed = HashSet::new();
        processed.insert(1);
        processed.insert(3);
        let result = get_unprocessed(&[1, 2, 3, 4], &processed);
        assert_eq!(result, vec![2, 4]);
    }

    #[test]
    fn diff_processed_no_overlap_returns_all() {
        let mut processed = HashSet::new();
        processed.insert(10);
        let result = get_unprocessed(&[1, 2], &processed);
        assert_eq!(result, vec![1, 2]);
    }

    // --- diff_cached_signal_group_ids tests ---

    fn make_signal_group(indicator_id: i32) -> SignalGroup {
        SignalGroup {
            indicator_id,
            long_signals: vec![],
            short_signals: vec![],
            operation_count: 0,
            instrument: "TEST".to_string(),
        }
    }

    #[test]
    fn diff_cached_empty_cached_returns_all_needed() {
        let result = diff_cached_signal_group_ids(&[], &[1, 2, 3]);
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn diff_cached_empty_needed_returns_empty() {
        let cached = vec![make_signal_group(1)];
        let result = diff_cached_signal_group_ids(&cached, &[]);
        assert!(result.is_empty());
    }

    #[test]
    fn diff_cached_full_overlap_returns_empty() {
        let cached = vec![make_signal_group(1), make_signal_group(2)];
        let result = diff_cached_signal_group_ids(&cached, &[1, 2]);
        assert!(result.is_empty());
    }

    #[test]
    fn diff_cached_partial_overlap_returns_missing() {
        let cached = vec![make_signal_group(1), make_signal_group(3)];
        let result = diff_cached_signal_group_ids(&cached, &[1, 2, 3, 4]);
        assert_eq!(result, vec![2, 4]);
    }

    #[test]
    fn diff_cached_no_overlap_returns_all() {
        let cached = vec![make_signal_group(10)];
        let result = diff_cached_signal_group_ids(&cached, &[1, 2]);
        assert_eq!(result, vec![1, 2]);
    }

    // --- merge_signal_groups tests ---

    #[test]
    fn merge_empty_cached_appends_all_new() {
        let new = vec![make_signal_group(1), make_signal_group(2)];
        let result = merge_signal_groups(vec![], new);
        let ids: Vec<i32> = result.iter().map(|sg| sg.indicator_id).collect();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn merge_empty_new_returns_cached_unchanged() {
        let cached = vec![make_signal_group(1)];
        let result = merge_signal_groups(cached, vec![]);
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].indicator_id, 1);
    }

    #[test]
    fn merge_duplicate_in_new_is_ignored() {
        let cached = vec![make_signal_group(1), make_signal_group(2)];
        let new = vec![make_signal_group(2)];
        let result = merge_signal_groups(cached, new);
        let ids: Vec<i32> = result.iter().map(|sg| sg.indicator_id).collect();
        assert_eq!(ids, vec![1, 2]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn merge_all_new_duplicates_all_ignored() {
        let cached = vec![make_signal_group(1), make_signal_group(2)];
        let new = vec![make_signal_group(1), make_signal_group(2)];
        let result = merge_signal_groups(cached, new);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn merge_partial_overlap_preserves_non_overlapping() {
        let cached = vec![make_signal_group(1), make_signal_group(2)];
        let new = vec![make_signal_group(2), make_signal_group(3)];
        let result = merge_signal_groups(cached, new);
        let mut ids: Vec<i32> = result.iter().map(|sg| sg.indicator_id).collect();
        ids.sort();
        assert_eq!(ids, vec![1, 2, 3]);
        assert_eq!(result.len(), 3);
    }

    #[test]
    fn merge_preserves_signal_data_not_just_ids() {
        let cached = vec![SignalGroup {
            indicator_id: 1,
            long_signals: vec![10, 20],
            short_signals: vec![30],
            operation_count: 3,
            instrument: "EURUSD".to_string(),
        }];
        let new = vec![SignalGroup {
            indicator_id: 2,
            long_signals: vec![40],
            short_signals: vec![50, 60],
            operation_count: 3,
            instrument: "EURUSD".to_string(),
        }];
        let result = merge_signal_groups(cached, new);
        assert_eq!(result.len(), 2);
        let sg1 = result.iter().find(|sg| sg.indicator_id == 1).unwrap();
        assert_eq!(sg1.long_signals, vec![10, 20]);
        assert_eq!(sg1.short_signals, vec![30]);
    }

    #[test]
    fn merge_cached_data_survives_when_new_has_same_id_with_different_data() {
        let cached = vec![SignalGroup {
            indicator_id: 1,
            long_signals: vec![10, 20],
            short_signals: vec![30],
            operation_count: 3,
            instrument: "EURUSD".to_string(),
        }];
        let new = vec![SignalGroup {
            indicator_id: 1,
            long_signals: vec![99, 99],
            short_signals: vec![99],
            operation_count: 999,
            instrument: "EURUSD".to_string(),
        }];
        let result = merge_signal_groups(cached, new);
        assert_eq!(result.len(), 1);
        let sg = &result[0];
        assert_eq!(sg.indicator_id, 1);
        // Cached data survives — new data with same id is ignored
        assert_eq!(sg.long_signals, vec![10, 20]);
        assert_eq!(sg.short_signals, vec![30]);
        assert_eq!(sg.operation_count, 3);
    }
}
