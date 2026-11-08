use rustc_hash::FxHashMap;

use rayon::prelude::*;

use crate::domain::annual_operation_count::annual_operation_count;
use crate::domain::operation_point::{OperationPoint, OperationPoints};
use crate::domain::performance_metrics::performance_metrics;
use crate::domain::signal_group::SignalGroup;
use crate::domain::strategy::Strategy;

pub fn create_strategies(
    signal_groups: &[SignalGroup],
    operation_points: &OperationPoints<'_>,
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

fn create_strategy(
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

    let total_count = long_operation_point_ids
        .len()
        .saturating_add(short_operation_point_ids.len());
    let annual_operation_count =
        annual_operation_count(total_count, start_timestamp, end_timestamp);

    let all_ops: Vec<&OperationPoint> = long_ops.into_iter().chain(short_ops).collect();
    let metrics = performance_metrics(&all_ops);
    let annual_roi = metrics.annual_roi(start_timestamp, end_timestamp);

    Strategy {
        instrument: signal_group.instrument.clone(),
        indicator_id: signal_group.indicator_id,
        annual_roi,
        max_draw_down: metrics.max_draw_down,
        annual_operation_count,
        long_operation_point_ids,
        short_operation_point_ids,
        money_management_strategy_id,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::fixtures::ONE_YEAR_SECS;

    fn op_point(id: i32, result: i32, timestamp: i64) -> OperationPoint {
        OperationPoint {
            instrument: "EURUSD".to_string(),
            result,
            tp: 30,
            sl: 20,
            risk: 0.02,
            money_management_strategy_id: 1,
            id,
            timestamp,
        }
    }

    fn group(indicator_id: i32, longs: Vec<i64>, shorts: Vec<i64>) -> SignalGroup {
        SignalGroup {
            indicator_id,
            instrument: "EURUSD".to_string(),
            long_signals: longs,
            short_signals: shorts,
        }
    }

    fn by_timestamp(points: &[OperationPoint]) -> FxHashMap<i64, &OperationPoint> {
        points.iter().map(|p| (p.timestamp, p)).collect()
    }

    // Sorted trades +30 (ts 1000), -20 (ts 2000), +30 (ts 3000) with
    // risk 0.02 / sl 20 compound 1.03 -> 1.0094 -> 1.0403: global ROI
    // 4.03 and max draw down 2.0 (the 1.03 -> 1.0094 dip). The span is
    // exactly one year, so annual ROI equals global ROI and the annual
    // operation count is 3 / 1 = 3.
    fn three_trade_strategy() -> Strategy {
        let long_ops = vec![op_point(11, 30, 1000), op_point(13, 30, 3000)];
        let short_ops = vec![op_point(12, -20, 2000)];
        let long_map = by_timestamp(&long_ops);
        let short_map = by_timestamp(&short_ops);
        let sg = group(7, vec![1000, 3000], vec![2000]);

        create_strategy(&sg, &long_map, &short_map, 1, 1000, 1000 + ONE_YEAR_SECS)
    }

    #[test]
    fn signals_resolve_to_matching_operation_points_in_signal_order() {
        let strategy = three_trade_strategy();

        assert_eq!(strategy.indicator_id, 7);
        assert_eq!(strategy.instrument, "EURUSD");
        assert_eq!(strategy.money_management_strategy_id, 1);
        assert_eq!(strategy.long_operation_point_ids, vec![11, 13]);
        assert_eq!(strategy.short_operation_point_ids, vec![12]);
    }

    #[test]
    fn metrics_wire_through_from_the_resolved_operations() {
        let strategy = three_trade_strategy();

        assert_eq!(strategy.annual_roi, 4.03);
        assert_eq!(strategy.max_draw_down, 2.0);
        assert_eq!(strategy.annual_operation_count, 3.0);
    }

    #[test]
    fn unresolved_signal_timestamps_are_dropped() {
        // Only ts 1000 resolves; ts 9999 has no operation point. Metrics
        // come from the resolved trades only: a single +30 -> ROI 3.0.
        let long_ops = vec![op_point(11, 30, 1000)];
        let long_map = by_timestamp(&long_ops);
        let short_map = by_timestamp(&[]);
        let sg = group(9, vec![1000, 9999], vec![]);

        let strategy = create_strategy(&sg, &long_map, &short_map, 1, 1000, 1000 + ONE_YEAR_SECS);

        assert_eq!(strategy.long_operation_point_ids, vec![11]);
        assert_eq!(strategy.annual_roi, 3.0);
        assert_eq!(strategy.max_draw_down, 0.0);
        assert_eq!(strategy.annual_operation_count, 1.0);
    }

    #[test]
    fn no_resolvable_signals_yield_zero_metrics() {
        let long_map = by_timestamp(&[]);
        let short_map = by_timestamp(&[]);
        let sg = group(5, vec![4242], vec![8888]);

        let strategy = create_strategy(&sg, &long_map, &short_map, 3, 0, ONE_YEAR_SECS);

        assert!(strategy.long_operation_point_ids.is_empty());
        assert!(strategy.short_operation_point_ids.is_empty());
        assert_eq!(strategy.annual_roi, 0.0);
        assert_eq!(strategy.max_draw_down, 0.0);
        assert_eq!(strategy.annual_operation_count, 0.0);
    }

    #[test]
    fn long_and_short_signals_at_shared_timestamp_resolve_to_their_own_side() {
        let long_ops = vec![op_point(21, 30, 1000)];
        let short_ops = vec![op_point(22, -20, 1000)];
        let long_map = by_timestamp(&long_ops);
        let short_map = by_timestamp(&short_ops);
        let sg = group(4, vec![1000], vec![1000]);

        let strategy = create_strategy(&sg, &long_map, &short_map, 1, 0, ONE_YEAR_SECS);

        assert_eq!(strategy.long_operation_point_ids, vec![21]);
        assert_eq!(strategy.short_operation_point_ids, vec![22]);
    }

    #[test]
    fn create_strategies_returns_one_strategy_per_group_in_input_order() {
        let long_ops = vec![op_point(31, 30, 1000)];
        let short_ops = vec![op_point(32, -20, 1000)];
        let operation_points = OperationPoints {
            long_operation_points: &long_ops,
            short_operation_points: &short_ops,
        };
        let indicator_ids: Vec<i32> = (1..=64).rev().collect();
        let groups: Vec<SignalGroup> = indicator_ids
            .iter()
            .map(|&id| group(id, vec![1000], vec![]))
            .collect();

        let strategies = create_strategies(&groups, &operation_points, 1, 0, ONE_YEAR_SECS);

        let result_ids: Vec<i32> = strategies.iter().map(|s| s.indicator_id).collect();
        assert_eq!(result_ids, indicator_ids);
        assert!(strategies
            .iter()
            .all(|s| s.long_operation_point_ids == vec![31]));
    }

    #[test]
    fn create_strategies_with_no_groups_returns_empty() {
        let long_ops = vec![op_point(31, 30, 1000)];
        let short_ops = vec![op_point(32, -20, 1000)];
        let operation_points = OperationPoints {
            long_operation_points: &long_ops,
            short_operation_points: &short_ops,
        };

        let strategies = create_strategies(&[], &operation_points, 1, 0, ONE_YEAR_SECS);

        assert!(strategies.is_empty());
    }
}
