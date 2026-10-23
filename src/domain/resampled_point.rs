use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct ResampledPointD1 {
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

    /// All points grouped by instrument.
    fn index_by_instrument(&self) -> anyhow::Result<FxHashMap<String, Vec<ResampledPointD1>>> {
        Ok(group_by_instrument(self.get_all()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(id: i32, instrument: &str) -> ResampledPointD1 {
        ResampledPointD1 {
            id,
            instrument: instrument.into(),
            open: 1.0,
            high: 1.1,
            low: 0.9,
            close: 1.05,
            volume: 1000,
            timestamp: 1000,
        }
    }

    #[test]
    fn group_by_instrument_groups_points() {
        let points = vec![point(1, "EURUSD"), point(2, "EURUSD"), point(3, "USDCAD")];
        let index = group_by_instrument(points);

        assert_eq!(index.len(), 2);
        assert_eq!(index["EURUSD"].len(), 2);
        assert_eq!(index["USDCAD"].len(), 1);
        assert_eq!(index["EURUSD"][0].id, 1);
        assert_eq!(index["EURUSD"][1].id, 2);
        assert_eq!(index["USDCAD"][0].id, 3);
    }
}
