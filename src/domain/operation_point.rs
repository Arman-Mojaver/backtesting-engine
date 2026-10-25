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
    pub timestamp: i64,
}

#[derive(Debug, Clone, Copy)]
pub struct OperationPoints<'a> {
    pub long_operation_points: &'a [OperationPoint],
    pub short_operation_points: &'a [OperationPoint],
}

impl OperationPoints<'_> {
    pub fn long_points_by_timestamp(&self) -> FxHashMap<i64, &OperationPoint> {
        self.long_operation_points
            .iter()
            .map(|p| (p.timestamp, p))
            .collect()
    }

    pub fn short_points_by_timestamp(&self) -> FxHashMap<i64, &OperationPoint> {
        self.short_operation_points
            .iter()
            .map(|p| (p.timestamp, p))
            .collect()
    }

    /// Long and short operation points carry identical timestamps (they are
    /// generated in lockstep upstream), so the range from the long side
    /// bounds both.
    pub fn start_end_timestamps(&self) -> Option<(i64, i64)> {
        let start = self
            .long_operation_points
            .iter()
            .map(|p| p.timestamp)
            .min()?;
        let end = self
            .long_operation_points
            .iter()
            .map(|p| p.timestamp)
            .max()?;
        Some((start, end))
    }
}

fn group_by_cell(points: Vec<OperationPoint>) -> FxHashMap<(String, i32), Vec<OperationPoint>> {
    points
        .into_iter()
        .fold(FxHashMap::default(), |mut acc, point| {
            acc.entry((point.instrument.clone(), point.money_management_strategy_id))
                .or_default()
                .push(point);
            acc
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

    fn make_point(timestamp: i64, id: i32, result: i32) -> OperationPoint {
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
        let long: Vec<OperationPoint> = vec![];
        let short: Vec<OperationPoint> = vec![];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &short,
        };
        assert!(ops.long_points_by_timestamp().is_empty());
        assert!(ops.short_points_by_timestamp().is_empty());
    }

    #[test]
    fn long_points_keyed_by_timestamp() {
        let long = vec![
            make_point(100, 1, 1),
            make_point(200, 2, 1),
            make_point(300, 3, 1),
        ];
        let short: Vec<OperationPoint> = vec![];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &short,
        };

        let map = ops.long_points_by_timestamp();
        assert_eq!(map.len(), 3);
        assert_eq!(map[&100].id, 1);
        assert_eq!(map[&200].id, 2);
        assert_eq!(map[&300].id, 3);
    }

    #[test]
    fn short_points_keyed_by_timestamp() {
        let long: Vec<OperationPoint> = vec![];
        let short = vec![make_point(50, 10, -1), make_point(150, 20, -1)];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &short,
        };

        let map = ops.short_points_by_timestamp();
        assert_eq!(map.len(), 2);
        assert_eq!(map[&50].id, 10);
        assert_eq!(map[&150].id, 20);
    }

    #[test]
    fn duplicate_timestamp_last_wins() {
        let long = vec![make_point(100, 1, 1), make_point(100, 2, 1)];
        let short: Vec<OperationPoint> = vec![];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &short,
        };

        let map = ops.long_points_by_timestamp();
        assert_eq!(map.len(), 1);
        assert_eq!(map[&100].id, 2);
    }

    #[test]
    fn long_and_short_independent() {
        let long = vec![make_point(100, 1, 1)];
        let short = vec![make_point(100, 10, -1)];
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
            make_point(300, 1, 1),
            make_point(100, 2, 1),
            make_point(200, 3, 1),
        ];
        let short: Vec<OperationPoint> = vec![];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &short,
        };
        let (start, end) = ops.start_end_timestamps().unwrap();
        assert_eq!(start, 100);
        assert_eq!(end, 300);
    }

    #[test]
    fn start_end_timestamps_single_point() {
        let long = vec![make_point(42, 1, 1)];
        let short: Vec<OperationPoint> = vec![];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &short,
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
                ..make_point(1000, 1, 1)
            },
            OperationPoint {
                instrument: "EURUSD".into(),
                money_management_strategy_id: 1,
                ..make_point(1001, 2, -1)
            },
            OperationPoint {
                instrument: "EURUSD".into(),
                money_management_strategy_id: 2,
                ..make_point(1000, 3, 1)
            },
            OperationPoint {
                instrument: "USDCAD".into(),
                money_management_strategy_id: 1,
                ..make_point(1000, 4, 1)
            },
        ];

        let index = group_by_cell(points);

        assert_eq!(index.len(), 3);
        assert_eq!(index[&("EURUSD".to_string(), 1)].len(), 2);
        assert_eq!(index[&("EURUSD".to_string(), 2)].len(), 1);
        assert_eq!(index[&("USDCAD".to_string(), 1)].len(), 1);
    }
}
