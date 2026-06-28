use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct OperationPoint {
    pub instrument: String,
    pub result: i32,
    pub tp: i32,
    pub risk: f64,
    pub money_management_strategy_id: i32,
    pub id: i32,
    pub sl: i32,
    pub timestamp: i32,
}

#[derive(Debug, Clone)]
pub struct OperationPoints {
    pub long_operation_points: Vec<OperationPoint>,
    pub short_operation_points: Vec<OperationPoint>,
}

impl OperationPoints {
    pub fn long_points_by_timestamp(&self) -> FxHashMap<i32, &OperationPoint> {
        self.long_operation_points
            .iter()
            .map(|p| (p.timestamp, p))
            .collect()
    }

    pub fn short_points_by_timestamp(&self) -> FxHashMap<i32, &OperationPoint> {
        self.short_operation_points
            .iter()
            .map(|p| (p.timestamp, p))
            .collect()
    }

    /// Returns the (start_timestamp, end_timestamp) range for long operation points.
    /// Both `long_operation_points` and `short_operation_points` share identical timestamps.
    pub fn start_end_timestamps(&self) -> (i32, i32) {
        let timestamps: Vec<i32> = self
            .long_operation_points
            .iter()
            .map(|p| p.timestamp)
            .collect();
        let start = *timestamps
            .iter()
            .min()
            .expect("long_operation_points should never be empty");
        let end = *timestamps
            .iter()
            .max()
            .expect("long_operation_points should never be empty");
        (start, end)
    }
}

pub trait LongOperationPointRepository {
    fn get_all(&self) -> Vec<OperationPoint>;
    fn get_by_mms_id_and_instrument(&self, mms_id: i32, instrument: &str) -> Vec<OperationPoint>;
}

pub trait ShortOperationPointRepository {
    fn get_all(&self) -> Vec<OperationPoint>;
    fn get_by_mms_id_and_instrument(&self, mms_id: i32, instrument: &str) -> Vec<OperationPoint>;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_point(timestamp: i32, id: i32, result: i32) -> OperationPoint {
        OperationPoint {
            instrument: "EURUSD".into(),
            result,
            tp: 100,
            sl: 50,
            risk: 0.02,
            money_management_strategy_id: 1,
            id,
            timestamp,
        }
    }

    #[test]
    fn empty_operations_yield_empty_hashmaps() {
        let ops = OperationPoints {
            long_operation_points: vec![],
            short_operation_points: vec![],
        };
        assert!(ops.long_points_by_timestamp().is_empty());
        assert!(ops.short_points_by_timestamp().is_empty());
    }

    #[test]
    fn long_points_keyed_by_timestamp() {
        let ops = OperationPoints {
            long_operation_points: vec![
                make_point(100, 1, 1),
                make_point(200, 2, 1),
                make_point(300, 3, 1),
            ],
            short_operation_points: vec![],
        };

        let map = ops.long_points_by_timestamp();
        assert_eq!(map.len(), 3);
        assert_eq!(map[&100].id, 1);
        assert_eq!(map[&200].id, 2);
        assert_eq!(map[&300].id, 3);
    }

    #[test]
    fn short_points_keyed_by_timestamp() {
        let ops = OperationPoints {
            long_operation_points: vec![],
            short_operation_points: vec![make_point(50, 10, -1), make_point(150, 20, -1)],
        };

        let map = ops.short_points_by_timestamp();
        assert_eq!(map.len(), 2);
        assert_eq!(map[&50].id, 10);
        assert_eq!(map[&150].id, 20);
    }

    #[test]
    fn duplicate_timestamp_last_wins() {
        let ops = OperationPoints {
            long_operation_points: vec![make_point(100, 1, 1), make_point(100, 2, 1)],
            short_operation_points: vec![],
        };

        let map = ops.long_points_by_timestamp();
        assert_eq!(map.len(), 1);
        assert_eq!(map[&100].id, 2);
    }

    #[test]
    fn long_and_short_independent() {
        let ops = OperationPoints {
            long_operation_points: vec![make_point(100, 1, 1)],
            short_operation_points: vec![make_point(100, 10, -1)],
        };

        let long_map = ops.long_points_by_timestamp();
        let short_map = ops.short_points_by_timestamp();
        assert_eq!(long_map.len(), 1);
        assert_eq!(short_map.len(), 1);
        assert_eq!(long_map[&100].id, 1);
        assert_eq!(short_map[&100].id, 10);
    }

    #[test]
    fn start_end_timestamps_unsorted() {
        let ops = OperationPoints {
            long_operation_points: vec![
                make_point(300, 1, 1),
                make_point(100, 2, 1),
                make_point(200, 3, 1),
            ],
            short_operation_points: vec![],
        };
        let (start, end) = ops.start_end_timestamps();
        assert_eq!(start, 100);
        assert_eq!(end, 300);
    }

    #[test]
    fn start_end_timestamps_single_point() {
        let ops = OperationPoints {
            long_operation_points: vec![make_point(42, 1, 1)],
            short_operation_points: vec![],
        };
        let (start, end) = ops.start_end_timestamps();
        assert_eq!(start, 42);
        assert_eq!(end, 42);
    }
}
