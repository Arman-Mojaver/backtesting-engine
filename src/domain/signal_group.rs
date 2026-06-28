use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalGroup {
    pub indicator_id: i32,
    pub long_signals: Vec<i64>,
    pub short_signals: Vec<i64>,
}

impl SignalGroup {
    pub fn operation_count(&self) -> usize {
        self.long_signals.len() + self.short_signals.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_group(long_count: usize, short_count: usize) -> SignalGroup {
        SignalGroup {
            indicator_id: 1,
            long_signals: vec![0; long_count],
            short_signals: vec![0; short_count],
        }
    }

    #[test]
    fn both_empty_returns_zero() {
        let group = make_group(0, 0);
        assert_eq!(group.operation_count(), 0);
    }

    #[test]
    fn only_longs_returns_long_count() {
        let group = make_group(5, 0);
        assert_eq!(group.operation_count(), 5);
    }

    #[test]
    fn only_shorts_returns_short_count() {
        let group = make_group(0, 3);
        assert_eq!(group.operation_count(), 3);
    }

    #[test]
    fn both_populated_returns_sum() {
        let group = make_group(4, 7);
        assert_eq!(group.operation_count(), 11);
    }
}
