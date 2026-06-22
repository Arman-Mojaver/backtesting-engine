use std::ops::RangeInclusive;

use rand::rngs::StdRng;
use rand::RngExt;
use rand::SeedableRng;

use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoint;
use crate::domain::strategy::Strategy;

/// Round an f64 value to 2 decimal places.
fn round_to_2(x: f64) -> f64 {
    (x * 100.0).round() / 100.0
}

/// Generate random strategies from database-sourced indicators and operation points.
pub fn generate_random_strategies(
    indicators: &[Indicator],
    long_ops: &[OperationPoint],
    short_ops: &[OperationPoint],
) -> Vec<Strategy> {
    let mut rng = rand::rng();
    let all_long_ids: Vec<i32> = long_ops.iter().map(|op| op.id).collect();
    let all_short_ids: Vec<i32> = short_ops.iter().map(|op| op.id).collect();

    indicators
        .iter()
        .enumerate()
        .map(|(i, indicator)| {
            let annual_roi = round_to_2(rng.random_range(-20.0..50.0));
            let max_draw_down = round_to_2(rng.random_range(5.0..40.0));
            let annual_operation_count = rng.random_range(10..=20) as f64;

            let long_ids = pick_random_subset(&mut rng, &all_long_ids, 0.7..=1.0);
            let short_ids = pick_random_subset(&mut rng, &all_short_ids, 0.7..=1.0);

            Strategy {
                strategy_id: i as i64 + 1,
                indicator_id: indicator.id,
                annual_roi,
                max_draw_down,
                annual_operation_count,
                long_operation_point_ids: long_ids,
                short_operation_point_ids: short_ids,
            }
        })
        .collect()
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

        let strategy = Strategy {
            strategy_id: i as i64 + 1,
            indicator_id: rng.random_range(1..=indicator_count as i32),
            annual_roi: round_to_2(rng.random_range(-30.0..60.0)),
            max_draw_down: round_to_2(rng.random_range(1.0..50.0)),
            annual_operation_count: round_to_2(rng.random_range(5.0..365.0)),
            long_operation_point_ids: pick_random_subset(&mut rng, &available_op_ids, 0.0..=1.0),
            short_operation_point_ids: pick_random_subset(&mut rng, &available_op_ids, 0.0..=1.0),
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
