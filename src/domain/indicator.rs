#[derive(Debug, Clone, PartialEq)]
pub enum IndicatorParameters {
    Macd {
        slow_type: String,
        slow_n: i32,
        fast_type: String,
        fast_n: i32,
    },
    Rsi {
        n: i32,
        high_threshold: i32,
        low_threshold: i32,
    },
}

#[derive(Debug, Clone)]
pub struct Indicator {
    pub id: i32,
    pub r#type: String,
    pub parameters: IndicatorParameters,
    pub identifier: String,
}

pub trait IndicatorRepository {
    fn get_all(&self) -> Vec<Indicator>;
    fn get_by_type(&self, indicator_type: &str) -> Vec<Indicator>;
}
