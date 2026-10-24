use std::collections::HashSet;

use anyhow::Context;
use log::debug;

use crate::application::signal_generator::create_signal_groups;
use crate::application::strategy_generator::create_strategies;
use crate::domain::indicator::Indicator;
use crate::domain::operation_point::{OperationPoint, OperationPoints};
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal_group::filter_signal_groups_by_annual_ops;
use crate::domain::strategy::{filter_strategies, GlobalStrategyFilter, Strategy};
use crate::domain::validation::combined_validation::validate_all;

pub fn run_strategies(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    long_operation_points: &[OperationPoint],
    short_operation_points: &[OperationPoint],
    money_management_strategy_id: i32,
    unprocessed_ids: &[i32],
) -> anyhow::Result<Vec<Strategy>> {
    validate_all(
        long_operation_points,
        short_operation_points,
        indicators,
        resampled_points,
        money_management_strategy_id,
    )
    .context("Validation failed")?;

    let operation_points = OperationPoints {
        long_operation_points,
        short_operation_points,
    };
    let (start_timestamp, end_timestamp) = operation_points
        .start_end_timestamps()
        .ok_or_else(|| anyhow::anyhow!("long operation points should never be empty"))?;

    let unprocessed_set: HashSet<i32> = unprocessed_ids.iter().copied().collect();
    let indicators_for_compute: Vec<Indicator> = indicators
        .iter()
        .filter(|indicator| unprocessed_set.contains(&indicator.id))
        .cloned()
        .collect();

    let signal_groups = create_signal_groups(&indicators_for_compute, resampled_points);

    let global_filter = GlobalStrategyFilter::default();
    let filtered_groups = filter_signal_groups_by_annual_ops(
        signal_groups,
        global_filter.min_operation_count,
        start_timestamp,
        end_timestamp,
    );
    debug!(
        "After annual operation count filter (min {}): {} signal groups",
        global_filter.min_operation_count,
        filtered_groups.len(),
    );

    let strategies = create_strategies(
        &filtered_groups,
        &operation_points,
        money_management_strategy_id,
        start_timestamp,
        end_timestamp,
    );

    let strategies = filter_strategies(strategies, &global_filter);
    debug!(
        "After strategy filter (min annual ROI {}, max drawdown {}): {} strategies",
        global_filter.min_annual_roi,
        global_filter.max_draw_down_max,
        strategies.len(),
    );

    Ok(strategies)
}
