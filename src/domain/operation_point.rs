use std::collections::hash_map::Entry;

use itertools::Itertools;
use rustc_hash::FxHashMap;

use crate::domain::cell::Cell;

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

/// Keys the map once per run of consecutive same-cell points rather than once
/// per point, so the instrument string is cloned per run.
fn group_by_cell(points: Vec<OperationPoint>) -> FxHashMap<Cell, Vec<OperationPoint>> {
    let mut points_by_cell = FxHashMap::default();
    let mut run: Vec<OperationPoint> = Vec::new();
    for point in points {
        if run.first().is_some_and(|first| !same_cell(first, &point)) {
            insert_run(&mut points_by_cell, std::mem::take(&mut run));
        }
        run.push(point);
    }
    insert_run(&mut points_by_cell, run);
    points_by_cell
}

fn same_cell(a: &OperationPoint, b: &OperationPoint) -> bool {
    a.money_management_strategy_id == b.money_management_strategy_id && a.instrument == b.instrument
}

fn insert_run(points_by_cell: &mut FxHashMap<Cell, Vec<OperationPoint>>, run: Vec<OperationPoint>) {
    let Some(first) = run.first() else {
        return;
    };
    let cell = Cell::new(first.instrument.clone(), first.money_management_strategy_id);
    match points_by_cell.entry(cell) {
        Entry::Occupied(mut cell_points) => cell_points.get_mut().extend(run),
        Entry::Vacant(cell_points) => {
            cell_points.insert(run);
        }
    }
}

pub trait LongOperationPointRepository {
    fn get_all(&mut self) -> anyhow::Result<Vec<OperationPoint>>;
    fn get_by_mms_id_and_instrument(
        &mut self,
        mms_id: i32,
        instrument: &str,
    ) -> anyhow::Result<Vec<OperationPoint>>;

    fn get_all_grouped_by_cell(&mut self) -> anyhow::Result<FxHashMap<Cell, Vec<OperationPoint>>> {
        Ok(group_by_cell(self.get_all()?))
    }
}

pub trait ShortOperationPointRepository {
    fn get_all(&mut self) -> anyhow::Result<Vec<OperationPoint>>;
    fn get_by_mms_id_and_instrument(
        &mut self,
        mms_id: i32,
        instrument: &str,
    ) -> anyhow::Result<Vec<OperationPoint>>;

    fn get_all_grouped_by_cell(&mut self) -> anyhow::Result<FxHashMap<Cell, Vec<OperationPoint>>> {
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
    fn start_end_timestamps_returns_none_when_long_points_empty() {
        let short = vec![op_point(1, -1, 100)];
        let ops = OperationPoints {
            long_operation_points: &[],
            short_operation_points: &short,
        };
        assert_eq!(ops.start_end_timestamps(), None);
    }

    #[test]
    fn start_end_timestamps_ignores_short_points() {
        let long = vec![op_point(1, 1, 200), op_point(2, 1, 300)];
        let short = vec![op_point(3, -1, 100), op_point(4, -1, 400)];
        let ops = OperationPoints {
            long_operation_points: &long,
            short_operation_points: &short,
        };
        assert_eq!(ops.start_end_timestamps(), Some((200, 300)));
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

        let grouped = group_by_cell(points);
        let ids_in = |instrument: &str, mms_id: i32| -> Vec<i32> {
            grouped[&Cell::new(instrument, mms_id)]
                .iter()
                .map(|p| p.id)
                .collect()
        };

        assert_eq!(grouped.len(), 3);
        assert_eq!(ids_in("EURUSD", 1), vec![1, 2]);
        assert_eq!(ids_in("EURUSD", 2), vec![3]);
        assert_eq!(ids_in("USDCAD", 1), vec![4]);
    }

    #[test]
    fn group_by_cell_returns_empty_map_for_no_points() {
        assert!(group_by_cell(vec![]).is_empty());
    }

    #[test]
    fn group_by_cell_merges_interleaved_cells_in_input_order() {
        let points = vec![
            OperationPoint {
                instrument: "EURUSD".into(),
                money_management_strategy_id: 1,
                ..op_point(1, 1, 1000)
            },
            OperationPoint {
                instrument: "USDCAD".into(),
                money_management_strategy_id: 1,
                ..op_point(2, 1, 1000)
            },
            OperationPoint {
                instrument: "EURUSD".into(),
                money_management_strategy_id: 1,
                ..op_point(3, 1, 1001)
            },
        ];

        let grouped = group_by_cell(points);
        let ids_in = |instrument: &str, mms_id: i32| -> Vec<i32> {
            grouped[&Cell::new(instrument, mms_id)]
                .iter()
                .map(|p| p.id)
                .collect()
        };

        assert_eq!(grouped.len(), 2);
        assert_eq!(ids_in("EURUSD", 1), vec![1, 3]);
        assert_eq!(ids_in("USDCAD", 1), vec![2]);
    }
}
