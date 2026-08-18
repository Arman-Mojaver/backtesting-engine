use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use rayon::prelude::*;

use crate::application::cell_output::CellOutput;
use crate::infrastructure::processed_manifest::ParquetProcessedManifest;
use crate::infrastructure::strategy_adapter::ParquetStrategyRepository;

pub struct StrategyPersistence {
    strategies: ParquetStrategyRepository,
    manifest: ParquetProcessedManifest,
}

impl StrategyPersistence {
    pub fn new(base_dir: String, root_dir: String) -> Self {
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
        let mut next_id = self.strategies.max_strategy_id()? + 1;

        // Sequential: assign a starting ID per cell and collect manifest entries.
        let mut jobs = Vec::with_capacity(cells.len());
        let mut manifest_entries = Vec::with_capacity(cells.len());
        for cell in cells {
            let start_id = next_id;
            next_id += cell.strategies.len() as i64;
            manifest_entries.push((
                cell.instrument.clone(),
                cell.mms_id,
                cell.processed_indicator_ids,
            ));
            jobs.push((start_id, cell.instrument, cell.mms_id, cell.strategies));
        }

        // Parallel: write each cell to its own partition.
        jobs.into_par_iter()
            .try_for_each(|(start_id, instrument, mms_id, strategies)| {
                self.strategies
                    .write_cell(&instrument, mms_id, &strategies, start_id)
            })?;

        // Commit (last): mark processed indicator IDs in the manifest.
        let processed_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
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
        let persistence = StrategyPersistence::new(
            base.to_string_lossy().to_string(),
            root.to_string_lossy().to_string(),
        );

        let cells = vec![
            CellOutput {
                instrument: "EURUSD".to_string(),
                mms_id: 390,
                strategies: vec![strategy("EURUSD", 2001, 390), strategy("EURUSD", 2002, 390)],
                processed_indicator_ids: vec![2001, 2002, 2003],
            },
            CellOutput {
                instrument: "USDJPY".to_string(),
                mms_id: 400,
                strategies: vec![strategy("USDJPY", 3001, 400)],
                processed_indicator_ids: vec![3001],
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
