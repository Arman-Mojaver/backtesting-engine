use crate::domain::strategy::Strategy;

pub struct CellOutput {
    pub instrument: String,
    pub mms_id: i32,
    pub strategies: Vec<Strategy>,
    pub computed_indicator_ids: Vec<i32>,
}
