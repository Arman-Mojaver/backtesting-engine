use std::collections::HashSet;

use crate::application::signal_generator::create_signal_groups;
use crate::application::source_data::SourceData;
use crate::application::strategy_generator::create_strategies;
use crate::application::strategy_runner_utils::get_unprocessed;
use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoints;
use crate::domain::signal_group::filter_signal_groups_by_annual_ops;
use crate::domain::strategy::{filter_strategies, GlobalStrategyFilter, Strategy};
use crate::domain::validation::combined_validation::{validate_all, CombinedValidationError};

pub fn run_strategies(
    data: SourceData,
    processed_ids: HashSet<i32>,
) -> anyhow::Result<Option<(Vec<Strategy>, Vec<i32>)>> {
    let SourceData {
        instrument,
        money_management_strategy_id: mms_id,
        indicators,
        resampled_points,
        long_operation_points,
        short_operation_points,
    } = data;

    validate_all(
        &long_operation_points,
        &short_operation_points,
        &indicators,
        &resampled_points,
        mms_id,
    )
    .map_err(|e: CombinedValidationError| anyhow::anyhow!("Validation failed: {:?}", e))?;

    let all_indicator_ids: Vec<i32> = indicators.iter().map(|i| i.id).collect();
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

    let operation_points = OperationPoints {
        long_operation_points,
        short_operation_points,
    };
    let (start_timestamp, end_timestamp) = operation_points.start_end_timestamps();

    let unprocessed_set: HashSet<i32> = unprocessed_ids.iter().copied().collect();
    let indicators_for_compute: Vec<Indicator> = indicators
        .into_iter()
        .filter(|indicator| unprocessed_set.contains(&indicator.id))
        .collect();

    let signal_groups = create_signal_groups(&indicators_for_compute, &resampled_points);

    let global_filter = GlobalStrategyFilter::default();
    let filtered_groups = filter_signal_groups_by_annual_ops(
        signal_groups,
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
