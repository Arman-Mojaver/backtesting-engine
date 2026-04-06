#[derive(Debug, Clone)]
pub struct Indicator {
    pub id: i32,
    pub r#type: String,
}

pub trait IndicatorRepository {
    fn get_all(&self) -> Vec<Indicator>;
}
