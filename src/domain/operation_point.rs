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

pub trait LongOperationPointRepository {
    fn get_all(&self) -> Vec<OperationPoint>;
}

pub trait ShortOperationPointRepository {
    fn get_all(&self) -> Vec<OperationPoint>;
}
