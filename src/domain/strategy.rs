use std::default::Default;

#[derive(Debug, Clone)]
pub struct Strategy {
    pub strategy_id: i64,
    pub indicator_id: i32,
    pub annual_roi: f64,
    pub max_draw_down: f64,
    pub annual_operation_count: f64,
    pub long_operation_point_ids: Vec<i32>,
    pub short_operation_point_ids: Vec<i32>,
    pub money_management_strategy_id: i32,
}

#[derive(Debug, Clone)]
pub struct GlobalStrategyFilter {
    /// Strategies with max_draw_down above this value are excluded.
    pub max_draw_down_max: f64,
    /// Strategies with annual_roi below this value are excluded.
    pub min_annual_roi: f64,
    /// Strategies with fewer annual operations are excluded.
    pub min_operation_count: f64,
}

impl Default for GlobalStrategyFilter {
    fn default() -> Self {
        Self {
            max_draw_down_max: 0.30,
            min_annual_roi: 0.10,
            min_operation_count: 10.0,
        }
    }
}

pub fn filter_strategies(
    strategies: Vec<Strategy>,
    filter: &GlobalStrategyFilter,
) -> Vec<Strategy> {
    strategies
        .into_iter()
        .filter(|s| {
            s.max_draw_down <= filter.max_draw_down_max
                && s.annual_roi >= filter.min_annual_roi
                && s.annual_operation_count >= filter.min_operation_count
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_strategy(id: i64, roi: f64, dd: f64, ops: f64) -> Strategy {
        Strategy {
            strategy_id: id,
            indicator_id: 1,
            annual_roi: roi,
            max_draw_down: dd,
            annual_operation_count: ops,
            long_operation_point_ids: vec![],
            short_operation_point_ids: vec![],
            money_management_strategy_id: 1,
        }
    }

    #[test]
    fn default_filter_has_reasonable_values() {
        let f = GlobalStrategyFilter::default();
        assert!(f.max_draw_down_max > 0.0);
        assert!(f.min_annual_roi > 0.0);
        assert!(f.min_operation_count > 0.0);
    }

    #[test]
    fn keeps_strategy_within_all_limits() {
        let s = make_strategy(1, 0.15, 0.20, 10.0);
        let filter = GlobalStrategyFilter::default();
        let result = filter_strategies(vec![s], &filter);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn excludes_high_drawdown() {
        let s = make_strategy(1, 0.15, 0.50, 10.0);
        let filter = GlobalStrategyFilter::default();
        let result = filter_strategies(vec![s], &filter);
        assert!(result.is_empty());
    }

    #[test]
    fn excludes_low_roi() {
        let s = make_strategy(1, 0.02, 0.20, 10.0);
        let filter = GlobalStrategyFilter {
            min_annual_roi: 0.05,
            ..GlobalStrategyFilter::default()
        };
        let result = filter_strategies(vec![s], &filter);
        assert!(result.is_empty());
    }

    #[test]
    fn excludes_low_operation_count() {
        let s = make_strategy(1, 0.15, 0.20, 2.0);
        let filter = GlobalStrategyFilter::default();
        let result = filter_strategies(vec![s], &filter);
        assert!(result.is_empty());
    }

    #[test]
    fn filters_mixed_bag() {
        let strategies = vec![
            make_strategy(1, 0.15, 0.20, 10.0), // passes
            make_strategy(2, 0.05, 0.20, 10.0), // low ROI
            make_strategy(3, 0.15, 0.50, 10.0), // high DD
            make_strategy(4, 0.15, 0.20, 3.0),  // low ops
            make_strategy(5, 0.25, 0.10, 20.0), // passes
        ];
        let filter = GlobalStrategyFilter::default();
        let result = filter_strategies(strategies, &filter);
        let ids: Vec<i64> = result.iter().map(|s| s.strategy_id).collect();
        assert_eq!(result.len(), 2);
        assert_eq!(ids, vec![1, 5]);
    }
}
