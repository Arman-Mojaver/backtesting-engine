use std::ops::RangeInclusive;

use rand::RngExt;

use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoint;
use crate::domain::strategy::Strategy;

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
            let annual_roi = rng.random_range(-20.0..50.0);
            let max_draw_down = rng.random_range(5.0..40.0);
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

    // Partial Fisher-Yates shuffle: only shuffle the first `count` elements
    let mut shuffled = items.to_vec();
    for i in 0..count.saturating_sub(1) {
        let j = rng.random_range(i..items.len());
        shuffled.swap(i, j);
    }
    shuffled[..count].to_vec()
}
