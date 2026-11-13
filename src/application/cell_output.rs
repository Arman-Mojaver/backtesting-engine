use crate::domain::cell::Cell;
use crate::domain::strategy::Strategy;

pub struct CellOutput {
    pub cell: Cell,
    pub strategies: Vec<Strategy>,
    pub computed_indicator_ids: Vec<i32>,
}
