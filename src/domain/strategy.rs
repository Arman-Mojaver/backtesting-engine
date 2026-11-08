#[derive(Debug, Clone)]
pub struct Strategy {
    pub instrument: String,
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
    pub max_draw_down_max: f64,
    pub min_annual_roi: f64,
    /// Enforced at the signal-group stage, where the count is known before
    /// the expensive metrics exist; kept here so all three thresholds live
    /// in one place.
    pub min_annual_operation_count: f64,
}

impl Default for GlobalStrategyFilter {
    fn default() -> Self {
        Self {
            max_draw_down_max: 20.0,
            min_annual_roi: 5.0,
            min_annual_operation_count: 10.0,
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
            s.max_draw_down <= filter.max_draw_down_max && s.annual_roi >= filter.min_annual_roi
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_strategy(indicator_id: i32, annual_roi: f64, max_draw_down: f64) -> Strategy {
        Strategy {
            instrument: "EURUSD".to_string(),
            indicator_id,
            annual_roi,
            max_draw_down,
            annual_operation_count: 10.0,
            long_operation_point_ids: vec![],
            short_operation_point_ids: vec![],
            money_management_strategy_id: 1,
        }
    }

    #[test]
    fn default_filter_encodes_baseline_quality_bar() {
        let filter = GlobalStrategyFilter::default();
        assert_eq!(filter.max_draw_down_max, 20.0);
        assert_eq!(filter.min_annual_roi, 5.0);
        assert_eq!(filter.min_annual_operation_count, 10.0);
    }

    #[test]
    fn keeps_strategy_within_all_limits() {
        let s = make_strategy(1, 15.0, 20.0);
        let filter = GlobalStrategyFilter::default();
        let result = filter_strategies(vec![s], &filter);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn excludes_high_drawdown() {
        let s = make_strategy(1, 15.0, 50.0);
        let filter = GlobalStrategyFilter::default();
        let result = filter_strategies(vec![s], &filter);
        assert!(result.is_empty());
    }

    #[test]
    fn excludes_low_roi() {
        let s = make_strategy(1, 2.0, 20.0);
        let filter = GlobalStrategyFilter {
            min_annual_roi: 5.0,
            ..GlobalStrategyFilter::default()
        };
        let result = filter_strategies(vec![s], &filter);
        assert!(result.is_empty());
    }

    #[test]
    fn filters_mixed_bag() {
        let strategies = vec![
            make_strategy(1, 15.0, 20.0),
            make_strategy(2, 3.0, 20.0),
            make_strategy(3, 15.0, 50.0),
            make_strategy(4, 15.0, 20.0),
            make_strategy(5, 25.0, 10.0),
        ];
        let filter = GlobalStrategyFilter::default();
        let result = filter_strategies(strategies, &filter);
        let indicator_ids: Vec<i32> = result.iter().map(|s| s.indicator_id).collect();
        assert_eq!(result.len(), 3);
        assert_eq!(indicator_ids, vec![1, 4, 5]);
    }
}
