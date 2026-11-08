use itertools::Itertools;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct OperationPoint {
    pub id: i32,
    pub instrument: String,
    pub money_management_strategy_id: i32,
    pub timestamp: i64,
    pub result: i32,
    pub tp: i32,
    pub sl: i32,
    pub risk: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct OperationPoints<'a> {
    pub long_operation_points: &'a [OperationPoint],
    pub short_operation_points: &'a [OperationPoint],
}

pub fn points_by_timestamp(points: &[OperationPoint]) -> FxHashMap<i64, &OperationPoint> {
    points.iter().map(|p| (p.timestamp, p)).collect()
}

impl OperationPoints<'_> {
    pub fn long_points_by_timestamp(&self) -> FxHashMap<i64, &OperationPoint> {
        points_by_timestamp(self.long_operation_points)
    }

    pub fn short_points_by_timestamp(&self) -> FxHashMap<i64, &OperationPoint> {
        points_by_timestamp(self.short_operation_points)
    }

    /// Long and short operation points carry identical timestamps (they are
    /// generated in lockstep upstream), so the range from the long side
    /// bounds both.
    pub fn start_end_timestamps(&self) -> Option<(i64, i64)> {
        self.long_operation_points
            .iter()
            .map(|p| p.timestamp)
            .minmax()
            .into_option()
    }
}

fn group_by_cell(points: Vec<OperationPoint>) -> FxHashMap<(String, i32), Vec<OperationPoint>> {
    points
        .into_iter()
        .fold(FxHashMap::default(), |mut points_by_cell, point| {
            points_by_cell
                .entry((point.instrument.clone(), point.money_management_strategy_id))
                .or_default()
                .push(point);
            points_by_cell
        })
}

pub trait LongOperationPointRepository {
    fn get_all(&self) -> anyhow::Result<Vec<OperationPoint>>;
    fn get_by_mms_id_and_instrument(
        &self,
        mms_id: i32,
        instrument: &str,
    ) -> anyhow::Result<Vec<OperationPoint>>;

    fn index_by_cell(&self) -> anyhow::Result<FxHashMap<(String, i32), Vec<OperationPoint>>> {
        Ok(group_by_cell(self.get_all()?))
    }
}

pub trait ShortOperationPointRepository {
    fn get_all(&self) -> anyhow::Result<Vec<OperationPoint>>;
    fn get_by_mms_id_and_instrument(
        &self,
        mms_id: i32,
        instrument: &str,
    ) -> anyhow::Result<Vec<OperationPoint>>;

    fn index_by_cell(&self) -> anyhow::Result<FxHashMap<(String, i32), Vec<OperationPoint>>> {
        Ok(group_by_cell(self.get_all()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::op_point;

    #[test]
    fn empty_operations_yield_empty_hashmaps() {
        let ops = OperationPoints {
            long_operation_points: &[],
            short_operation_points: &[],
        };
        assert!(ops.long_points_by_timestamp().is_empty());
        assert!(ops.short_points_by_timestamp().is_empty());
    }

    #[test]
    fn long_points_keyed_by_timestamp() {
        let long = vec![
            op_point(1, 1, 100),
            op_point(2, 1, 200),
            op_point(3, 1, 300),
        ];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &[],
        };

        let map = ops.long_points_by_timestamp();
        assert_eq!(map.len(), 3);
        assert_eq!(map[&100].id, 1);
        assert_eq!(map[&200].id, 2);
        assert_eq!(map[&300].id, 3);
    }

    #[test]
    fn short_points_keyed_by_timestamp() {
        let short = vec![op_point(10, -1, 50), op_point(20, -1, 150)];
        let ops = OperationPoints {
            long_operation_points: &[],
            short_operation_points: &short,
        };

        let map = ops.short_points_by_timestamp();
        assert_eq!(map.len(), 2);
        assert_eq!(map[&50].id, 10);
        assert_eq!(map[&150].id, 20);
    }

    #[test]
    fn duplicate_timestamp_last_wins() {
        let long = vec![op_point(1, 1, 100), op_point(2, 1, 100)];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &[],
        };

        let map = ops.long_points_by_timestamp();
        assert_eq!(map.len(), 1);
        assert_eq!(map[&100].id, 2);
    }

    #[test]
    fn long_and_short_independent() {
        let long = vec![op_point(1, 1, 100)];
        let short = vec![op_point(10, -1, 100)];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &short,
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
        let long = vec![
            op_point(1, 1, 300),
            op_point(2, 1, 100),
            op_point(3, 1, 200),
        ];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &[],
        };
        let (start, end) = ops.start_end_timestamps().unwrap();
        assert_eq!(start, 100);
        assert_eq!(end, 300);
    }

    #[test]
    fn start_end_timestamps_single_point() {
        let long = vec![op_point(1, 1, 42)];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &[],
        };
        let (start, end) = ops.start_end_timestamps().unwrap();
        assert_eq!(start, 42);
        assert_eq!(end, 42);
    }

    #[test]
    fn group_by_cell_groups_points() {
        let points = vec![
            OperationPoint {
                instrument: "EURUSD".into(),
                money_management_strategy_id: 1,
                ..op_point(1, 1, 1000)
            },
            OperationPoint {
                instrument: "EURUSD".into(),
                money_management_strategy_id: 1,
                ..op_point(2, -1, 1001)
            },
            OperationPoint {
                instrument: "EURUSD".into(),
                money_management_strategy_id: 2,
                ..op_point(3, 1, 1000)
            },
            OperationPoint {
                instrument: "USDCAD".into(),
                money_management_strategy_id: 1,
                ..op_point(4, 1, 1000)
            },
        ];

        let index = group_by_cell(points);
        let ids_in = |instrument: &str, mms_id: i32| -> Vec<i32> {
            index[&(instrument.to_string(), mms_id)]
                .iter()
                .map(|p| p.id)
                .collect()
        };

        assert_eq!(index.len(), 3);
        assert_eq!(ids_in("EURUSD", 1), vec![1, 2]);
        assert_eq!(ids_in("EURUSD", 2), vec![3]);
        assert_eq!(ids_in("USDCAD", 1), vec![4]);
    }
}
