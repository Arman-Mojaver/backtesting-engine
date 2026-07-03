use rand::RngExt;

use crate::domain::indicator::Indicator;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal_group::SignalGroup;

/// Create a `SignalGroup` for each indicator, populated with random timestamps
/// drawn from `resampled_points` (excluding the first and last 200 entries).
///
/// Timestamps are intercalated between long and short (first → long, second →
/// short, third → long, …) and never repeated across the two vectors.
///
/// `count` controls how many timestamps land in each of long and short
/// (default 100).
pub fn create_signals(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    count: usize,
) -> Vec<SignalGroup> {
    let mut rng = rand::rng();

    // Collect eligible timestamps, skipping the first 200 and last 200.
    let mut eligible: Vec<i64> = Vec::new();
    let len = resampled_points.len();
    if len > 400 {
        for rp in resampled_points.iter().skip(200).take(len - 400) {
            eligible.push(rp.timestamp as i64);
        }
    }

    let mut signal_groups: Vec<SignalGroup> = Vec::new();

    for indicator in indicators {
        let mut long_signals: Vec<i64> = Vec::new();
        let mut short_signals: Vec<i64> = Vec::new();

        // Pick random indices from eligible timestamps, without repeating.
        let mut used_indices: Vec<usize> = Vec::new();
        let total_needed = count * 2;

        while long_signals.len() < count || short_signals.len() < count {
            if eligible.is_empty() {
                break;
            }

            let index = rng.random_range(0..eligible.len());

            // Skip if this index was already used.
            if used_indices.contains(&index) {
                continue;
            }
            used_indices.push(index);

            let timestamp = eligible[index];

            // Intercalate: first goes to long, second to short, third to long, ...
            if long_signals.len() <= short_signals.len() {
                long_signals.push(timestamp);
            } else {
                short_signals.push(timestamp);
            }

            if used_indices.len() >= total_needed {
                break;
            }
        }

        signal_groups.push(SignalGroup {
            indicator_id: indicator.id,
            long_signals,
            short_signals,
        });
    }

    signal_groups
}
