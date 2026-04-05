#[derive(Debug, Clone)]
pub struct ResampledPointD1 {
    pub id: i32,
    pub instrument: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i32,
    pub timestamp: i32,
}

pub trait ResampledPointD1Repository {
    fn get_all(&self) -> Vec<ResampledPointD1>;
}
