use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Cell {
    pub instrument: String,
    pub mms_id: i32,
}

impl Cell {
    pub fn new(instrument: impl Into<String>, mms_id: i32) -> Self {
        Self {
            instrument: instrument.into(),
            mms_id,
        }
    }
}

impl fmt::Display for Cell {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (mms_id={})", self.instrument, self.mms_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays_instrument_and_mms_id() {
        assert_eq!(Cell::new("EURUSD", 7).to_string(), "EURUSD (mms_id=7)");
    }

    #[test]
    fn sorts_by_instrument_then_mms_id() {
        let mut cells = vec![
            Cell::new("GBPUSD", 1),
            Cell::new("EURUSD", 2),
            Cell::new("EURUSD", 1),
        ];

        cells.sort();

        assert_eq!(
            cells,
            vec![
                Cell::new("EURUSD", 1),
                Cell::new("EURUSD", 2),
                Cell::new("GBPUSD", 1),
            ]
        );
    }
}
