use rustc_hash::FxHashSet;

use crate::domain::indicator::Indicator;

pub fn get_unprocessed(indicators: &[Indicator], processed_ids: &FxHashSet<i32>) -> Vec<i32> {
    indicators
        .iter()
        .map(|indicator| indicator.id)
        .filter(|id| !processed_ids.contains(id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::indicator::IndicatorParameters;

    fn indicators(ids: &[i32]) -> Vec<Indicator> {
        ids.iter()
            .map(|&id| Indicator {
                id,
                parameters: IndicatorParameters::AwesomeOscillator,
            })
            .collect()
    }

    #[test]
    fn diff_processed_empty_both_returns_empty() {
        let result = get_unprocessed(&[], &FxHashSet::default());
        assert!(result.is_empty());
    }

    #[test]
    fn diff_processed_empty_processed_returns_all() {
        let result = get_unprocessed(&indicators(&[1, 2, 3]), &FxHashSet::default());
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn diff_processed_full_overlap_returns_empty() {
        let mut processed = FxHashSet::default();
        processed.insert(1);
        processed.insert(2);
        let result = get_unprocessed(&indicators(&[1, 2]), &processed);
        assert!(result.is_empty());
    }

    #[test]
    fn diff_processed_partial_overlap_returns_unprocessed() {
        let mut processed = FxHashSet::default();
        processed.insert(1);
        processed.insert(3);
        let result = get_unprocessed(&indicators(&[1, 2, 3, 4]), &processed);
        assert_eq!(result, vec![2, 4]);
    }

    #[test]
    fn diff_processed_no_overlap_returns_all() {
        let mut processed = FxHashSet::default();
        processed.insert(10);
        let result = get_unprocessed(&indicators(&[1, 2]), &processed);
        assert_eq!(result, vec![1, 2]);
    }
}
