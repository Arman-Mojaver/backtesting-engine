use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use rayon::prelude::*;

use crate::application::cell_output::CellOutput;
use crate::infrastructure::processed_manifest::{unix_now_secs, ParquetProcessedManifest};
use crate::infrastructure::strategy_adapter::{ParquetStrategyRepository, StrategyRow};

pub struct CellOutputDto {
    pub instrument: String,
    pub mms_id: i32,
    pub strategy_rows: Vec<StrategyRow>,
    pub processed_indicator_ids: Vec<i32>,
}

impl CellOutputDto {
    pub(crate) fn manifest_entry(&self) -> (String, i32, Vec<i32>) {
        (
            self.instrument.clone(),
            self.mms_id,
            self.processed_indicator_ids.clone(),
        )
    }
}

pub struct StrategyPersistence {
    strategies: ParquetStrategyRepository,
    manifest: ParquetProcessedManifest,
}

impl StrategyPersistence {
    pub const fn new(base_dir: PathBuf, root_dir: PathBuf) -> Self {
        Self {
            strategies: ParquetStrategyRepository::new(base_dir),
            manifest: ParquetProcessedManifest::new(root_dir),
        }
    }

    pub fn read_processed_ids_per_cell(
        &self,
    ) -> anyhow::Result<HashMap<(String, i32), HashSet<i32>>> {
        self.manifest.read_processed_ids_per_cell()
    }

    pub fn persist_cells(&self, cells: Vec<CellOutput>) -> anyhow::Result<()> {
        let mut next_id = self.strategies.max_strategy_id()?.saturating_add(1);
        let mut dto_cells: Vec<CellOutputDto> = Vec::with_capacity(cells.len());
        for cell in cells {
            let mut strategy_rows = Vec::with_capacity(cell.strategies.len());
            for strategy in cell.strategies {
                strategy_rows.push(StrategyRow::from_strategy(&strategy, next_id));
                next_id = next_id.saturating_add(1);
            }
            dto_cells.push(CellOutputDto {
                instrument: cell.instrument,
                mms_id: cell.mms_id,
                strategy_rows,
                processed_indicator_ids: cell.computed_indicator_ids,
            });
        }

        let manifest_entries: Vec<(String, i32, Vec<i32>)> = dto_cells
            .iter()
            .map(CellOutputDto::manifest_entry)
            .collect();

        dto_cells.into_par_iter().try_for_each(|c| {
            self.strategies
                .write_rows(&c.instrument, c.mms_id, c.strategy_rows)
        })?;

        let processed_at = unix_now_secs()?;
        self.manifest.append_all(&manifest_entries, processed_at)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::strategy::Strategy;
    use tempfile::TempDir;

    fn strategy(instrument: &str, indicator_id: i32, mms_id: i32) -> Strategy {
        Strategy {
            instrument: instrument.to_string(),
            indicator_id,
            annual_roi: 1.0,
            max_draw_down: 1.0,
            annual_operation_count: 1.0,
            long_operation_point_ids: vec![],
            short_operation_point_ids: vec![],
            money_management_strategy_id: mms_id,
        }
    }

    #[test]
    fn persist_cells_writes_partitions_and_manifest() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let base = root.join("strategy");
        let persistence = StrategyPersistence::new(base.clone(), root);

        let cells = vec![
            CellOutput {
                instrument: "EURUSD".to_string(),
                mms_id: 390,
                strategies: vec![strategy("EURUSD", 2001, 390), strategy("EURUSD", 2002, 390)],
                computed_indicator_ids: vec![2001, 2002, 2003],
            },
            CellOutput {
                instrument: "USDJPY".to_string(),
                mms_id: 400,
                strategies: vec![strategy("USDJPY", 3001, 400)],
                computed_indicator_ids: vec![3001],
            },
        ];

        persistence.persist_cells(cells).unwrap();

        // Both partitions written.
        assert!(base
            .join("instrument=EURUSD/money_management_strategy_id=390/strategies.parquet")
            .exists());
        assert!(base
            .join("instrument=USDJPY/money_management_strategy_id=400/strategies.parquet")
            .exists());

        // Manifest records the full processed set, including 2003 (which had no strategy).
        let processed = persistence.read_processed_ids_per_cell().unwrap();
        let eur = processed.get(&("EURUSD".to_string(), 390)).unwrap();
        assert_eq!(eur.len(), 3);
        assert!(eur.contains(&2001));
        assert!(eur.contains(&2002));
        assert!(eur.contains(&2003));

        let usd = processed.get(&("USDJPY".to_string(), 400)).unwrap();
        assert_eq!(usd.len(), 1);
        assert!(usd.contains(&3001));
    }
}
