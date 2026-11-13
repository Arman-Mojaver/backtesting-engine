#[derive(Debug, Clone)]
pub struct Strategy {
    /// Intentional: unread by the engine and not a stored column, since the
    /// partition path already holds it; kept to trace a strategy to its instrument.
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
    /// Intentional: the name holds two different "max". `draw_down_max` is the
    /// metric itself, the maximum of the draw-down series. The leading `max`
    /// is the project-wide (global) upper limit that metric may reach.
    pub max_draw_down_max: f64,
    pub min_annual_roi: f64,
    /// Checked twice: first on raw signal counts, a free upper bound that
    /// never drops a passing group, then on matched trades, the count that is
    /// stored, before the expensive metrics are computed.
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

impl GlobalStrategyFilter {
    pub fn admits_activity(&self, annual_operation_count: f64) -> bool {
        annual_operation_count >= self.min_annual_operation_count
    }

    pub fn admits_performance(&self, annual_roi: f64, max_draw_down: f64) -> bool {
        annual_roi >= self.min_annual_roi && max_draw_down <= self.max_draw_down_max
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_filter_encodes_baseline_quality_bar() {
        let filter = GlobalStrategyFilter::default();
        assert_eq!(filter.max_draw_down_max, 20.0);
        assert_eq!(filter.min_annual_roi, 5.0);
        assert_eq!(filter.min_annual_operation_count, 10.0);
    }

    #[test]
    fn admits_activity_at_minimum() {
        let filter = GlobalStrategyFilter::default();
        assert!(filter.admits_activity(10.0));
    }

    #[test]
    fn rejects_activity_just_below_minimum() {
        let filter = GlobalStrategyFilter::default();
        assert!(!filter.admits_activity(9.99));
    }

    #[test]
    fn admits_activity_just_above_minimum() {
        let filter = GlobalStrategyFilter::default();
        assert!(filter.admits_activity(10.01));
    }

    #[test]
    fn admits_performance_with_annual_roi_at_minimum() {
        let filter = GlobalStrategyFilter::default();
        assert!(filter.admits_performance(5.0, 10.0));
    }

    #[test]
    fn rejects_performance_with_annual_roi_just_below_minimum() {
        let filter = GlobalStrategyFilter::default();
        assert!(!filter.admits_performance(4.99, 10.0));
    }

    #[test]
    fn admits_performance_with_annual_roi_just_above_minimum() {
        let filter = GlobalStrategyFilter::default();
        assert!(filter.admits_performance(5.01, 10.0));
    }

    #[test]
    fn admits_performance_with_draw_down_at_maximum() {
        let filter = GlobalStrategyFilter::default();
        assert!(filter.admits_performance(15.0, 20.0));
    }

    #[test]
    fn admits_performance_with_draw_down_just_below_maximum() {
        let filter = GlobalStrategyFilter::default();
        assert!(filter.admits_performance(15.0, 19.99));
    }

    #[test]
    fn rejects_performance_with_draw_down_just_above_maximum() {
        let filter = GlobalStrategyFilter::default();
        assert!(!filter.admits_performance(15.0, 20.01));
    }

    #[test]
    fn rejects_performance_with_nan_metrics() {
        let filter = GlobalStrategyFilter::default();
        assert!(!filter.admits_performance(f64::NAN, 10.0));
        assert!(!filter.admits_performance(15.0, f64::NAN));
    }
}
