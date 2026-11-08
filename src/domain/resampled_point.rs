use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct ResampledPointD1 {
    // Intentional: unread by the engine; kept to trace a point back to its DB row.
    pub id: i32,
    pub instrument: String,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: i32,
    pub timestamp: i64,
}

fn group_by_instrument(points: Vec<ResampledPointD1>) -> FxHashMap<String, Vec<ResampledPointD1>> {
    points
        .into_iter()
        .fold(FxHashMap::default(), |mut acc, point| {
            acc.entry(point.instrument.clone()).or_default().push(point);
            acc
        })
}

pub trait ResampledPointD1Repository {
    fn get_all(&self) -> anyhow::Result<Vec<ResampledPointD1>>;
    fn get_by_instrument(&self, instrument: &str) -> anyhow::Result<Vec<ResampledPointD1>>;

    fn index_by_instrument(&self) -> anyhow::Result<FxHashMap<String, Vec<ResampledPointD1>>> {
        Ok(group_by_instrument(self.get_all()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(id: i32, instrument: &str, timestamp: i64) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: instrument.into(),
            open: 1.0,
            high: 1.1,
            low: 0.9,
            close: 1.05,
            volume: 1000,
            timestamp,
        }
    }

    #[test]
    fn group_by_instrument_keeps_chronological_order_within_instrument() {
        let points = vec![
            point(1, "EURUSD", 1000),
            point(2, "USDCAD", 2000),
            point(3, "EURUSD", 3000),
        ];
        let index = group_by_instrument(points);

        let timestamps = |instrument: &str| -> Vec<i64> {
            index[instrument]
                .iter()
                .map(|point| point.timestamp)
                .collect()
        };
        assert_eq!(index.len(), 2);
        assert_eq!(timestamps("EURUSD"), vec![1000, 3000]);
        assert_eq!(timestamps("USDCAD"), vec![2000]);
    }
}
