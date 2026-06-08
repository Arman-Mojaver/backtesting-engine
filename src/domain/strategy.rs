#[derive(Debug, Clone)]
pub struct Strategy {
    pub strategy_id: i64,
    pub indicator_id: i32,
    pub annual_roi: f64,
    pub max_draw_down: f64,
    pub annual_operation_count: f64,
    pub long_operation_point_ids: Vec<i32>,
    pub short_operation_point_ids: Vec<i32>,
}
