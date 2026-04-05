#[derive(Debug, Clone)]
pub struct Indicator {
    pub id: i32,
}

pub trait IndicatorRepository {
    fn get_all(&self) -> Vec<Indicator>;
}
