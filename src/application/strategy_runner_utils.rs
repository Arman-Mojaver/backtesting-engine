use std::collections::HashSet;

pub fn get_unprocessed(all_indicator_ids: &[i32], processed_ids: &HashSet<i32>) -> Vec<i32> {
    all_indicator_ids
        .iter()
        .copied()
        .filter(|id| !processed_ids.contains(id))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
