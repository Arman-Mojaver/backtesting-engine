use std::ops::RangeInclusive;

use rand::rngs::StdRng;
use rand::RngExt;
use rand::SeedableRng;
use rustc_hash::FxHashMap;

use crate::domain::annual_operation_count::annual_operation_count;
use crate::domain::operation_point::{OperationPoint, OperationPoints};
use crate::domain::signal_group::SignalGroup;
use crate::domain::strategy::Strategy;

/// Round an f64 value to 2 decimal places.
fn round_to_2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Hardcoded placeholder metrics — to be computed from operation points later.
const PLACEHOLDER_ANNUAL_ROI: f64 = 0.15;
const PLACEHOLDER_MAX_DRAW_DOWN: f64 = 0.20;

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
        .iter()
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
    let long_operation_point_ids: Vec<i32> = signal_group
        .long_signals
        .iter()
        .filter_map(|ts| long_map.get(ts).map(|op| op.id))
        .collect();

    let short_operation_point_ids: Vec<i32> = signal_group
        .short_signals
        .iter()
        .filter_map(|ts| short_map.get(ts).map(|op| op.id))
        .collect();

    let total_count = signal_group.operation_count();
    let annual_operation_count =
        annual_operation_count(total_count, start_timestamp, end_timestamp);

    Strategy {
        indicator_id: signal_group.indicator_id,
        annual_roi: PLACEHOLDER_ANNUAL_ROI,
        max_draw_down: PLACEHOLDER_MAX_DRAW_DOWN,
        annual_operation_count,
        long_operation_point_ids,
        short_operation_point_ids,
        money_management_strategy_id,
    }
}

/// Generate N purely-synthetic random strategies distributed across
/// (instrument, mms_id) partitions. No DB needed.
pub fn generate_random_strategies_bulk(
    count: usize,
    instruments: &[String],
    mms_count: usize,
    max_op_points: usize,
    seed: Option<u64>,
) -> Vec<(String, i32, Vec<Strategy>)> {
    let mut rng: StdRng = match seed {
        Some(s) => StdRng::seed_from_u64(s),
        None => StdRng::from_rng(&mut rand::rng()),
    };

    let mut partitions: Vec<(String, i32)> = Vec::new();
    for inst in instruments {
        for mms_id in 1..=mms_count {
            partitions.push((inst.clone(), mms_id as i32));
        }
    }

    if partitions.is_empty() {
        return Vec::new();
    }

    let available_op_ids: Vec<i32> = (1..=max_op_points as i32).collect();
    let indicator_count = (count / 10).clamp(100, 1000);

    let mut partition_strategies: Vec<Vec<Strategy>> = vec![Vec::new(); partitions.len()];

    for i in 0..count {
        let idx = i % partitions.len();
        let money_management_strategy_id = partitions[idx].1;

        let strategy = Strategy {
            indicator_id: rng.random_range(1..=indicator_count as i32),
            annual_roi: round_to_2(rng.random_range(-30.0..60.0)),
            max_draw_down: round_to_2(rng.random_range(1.0..50.0)),
            annual_operation_count: round_to_2(rng.random_range(5.0..365.0)),
            long_operation_point_ids: pick_random_subset(&mut rng, &available_op_ids, 0.0..=1.0),
            short_operation_point_ids: pick_random_subset(&mut rng, &available_op_ids, 0.0..=1.0),
            money_management_strategy_id,
        };

        partition_strategies[idx].push(strategy);
    }

    partitions
        .into_iter()
        .zip(partition_strategies)
        .map(|((inst, mms), strs)| (inst, mms, strs))
        .collect()
}

fn pick_random_subset(
    rng: &mut impl RngExt,
    items: &[i32],
    ratio_range: RangeInclusive<f64>,
) -> Vec<i32> {
    if items.is_empty() {
        return Vec::new();
    }
    let ratio = rng.random_range(ratio_range);
    let count = ((items.len() as f64) * ratio).round() as usize;
    let count = count.clamp(1, items.len());

    let mut shuffled = items.to_vec();
    for i in 0..count.saturating_sub(1) {
        let j = rng.random_range(i..items.len());
        shuffled.swap(i, j);
    }
    shuffled[..count].to_vec()
}
