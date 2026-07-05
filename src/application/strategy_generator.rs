use rustc_hash::FxHashMap;

use rayon::prelude::*;

use crate::domain::annual_operation_count::annual_operation_count;
use crate::domain::annual_roi::annual_roi;
use crate::domain::global_roi::global_roi;
use crate::domain::max_draw_down::max_draw_down;
use crate::domain::operation_point::{OperationPoint, OperationPoints};
use crate::domain::signal_group::SignalGroup;
use crate::domain::strategy::Strategy;

pub fn create_strategies(
    signal_groups: &[SignalGroup],
    operation_points: &OperationPoints,
    money_management_strategy_id: i32,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Vec<Strategy> {
    let long_map = operation_points.long_points_by_timestamp();
    let short_map = operation_points.short_points_by_timestamp();

    signal_groups
        .par_iter()
        .map(|sg| {
            create_strategy(
                sg,
                &long_map,
                &short_map,
                money_management_strategy_id,
                start_timestamp,
                end_timestamp,
            )
        })
        .collect()
}

pub fn create_strategy(
    signal_group: &SignalGroup,
    long_map: &FxHashMap<i64, &OperationPoint>,
    short_map: &FxHashMap<i64, &OperationPoint>,
    money_management_strategy_id: i32,
    start_timestamp: i64,
    end_timestamp: i64,
) -> Strategy {
    let long_ops: Vec<&OperationPoint> = signal_group
        .long_signals
        .iter()
        .filter_map(|ts| long_map.get(ts).copied())
        .collect();

    let short_ops: Vec<&OperationPoint> = signal_group
        .short_signals
        .iter()
        .filter_map(|ts| short_map.get(ts).copied())
        .collect();

    let long_operation_point_ids: Vec<i32> = long_ops.iter().map(|op| op.id).collect();
    let short_operation_point_ids: Vec<i32> = short_ops.iter().map(|op| op.id).collect();

    let total_count = long_operation_point_ids.len() + short_operation_point_ids.len();
    let annual_operation_count =
        annual_operation_count(total_count, start_timestamp, end_timestamp);

    let mut all_ops: Vec<&OperationPoint> = long_ops.into_iter().chain(short_ops).collect();
    all_ops.sort_by_key(|op| op.timestamp);
    let max_draw_down = max_draw_down(&all_ops);

    let global_roi = global_roi(&all_ops);
    let annual_roi = annual_roi(start_timestamp, end_timestamp, global_roi);

    Strategy {
        indicator_id: signal_group.indicator_id,
        annual_roi,
        max_draw_down,
        annual_operation_count,
        long_operation_point_ids,
        short_operation_point_ids,
        money_management_strategy_id,
    }
}
