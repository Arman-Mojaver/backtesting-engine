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
        if cells.is_empty() {
            return Ok(());
        }

        // strategy_id is persistence-assigned: new ids start from the global max
        // across all partitions, so they stay unique across the whole tree.
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
    use arrow::array::AsArray;
    use arrow::datatypes::Int64Type;
    use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
    use std::fs::File;
    use std::path::Path;
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

    fn cell(instrument: &str, mms_id: i32, indicator_ids: &[i32]) -> CellOutput {
        CellOutput {
            instrument: instrument.to_string(),
            mms_id,
            strategies: indicator_ids
                .iter()
                .map(|&indicator_id| strategy(instrument, indicator_id, mms_id))
                .collect(),
            computed_indicator_ids: indicator_ids.to_vec(),
        }
    }

    fn strategy_ids(base: &Path, instrument: &str, mms_id: i32) -> Vec<i64> {
        let path = base.join(format!(
            "instrument={instrument}/money_management_strategy_id={mms_id}/strategies.parquet"
        ));
        let reader = ParquetRecordBatchReaderBuilder::try_new(File::open(path).unwrap())
            .unwrap()
            .build()
            .unwrap();
        reader
            .flat_map(|batch| {
                batch
                    .unwrap()
                    .column_by_name("strategy_id")
                    .unwrap()
                    .as_primitive::<Int64Type>()
                    .values()
                    .to_vec()
            })
            .collect()
    }

    fn new_persistence() -> (TempDir, PathBuf, StrategyPersistence) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let base = root.join("strategy");
        let persistence = StrategyPersistence::new(base.clone(), root);
        (tmp, base, persistence)
    }

    #[test]
    fn first_persist_starts_ids_at_one() {
        let (_tmp, base, persistence) = new_persistence();

        persistence
            .persist_cells(vec![cell("EURUSD", 390, &[2001, 2002])])
            .unwrap();

        assert_eq!(strategy_ids(&base, "EURUSD", 390), vec![1, 2]);
    }

    #[test]
    fn ids_are_contiguous_within_a_cell() {
        let (_tmp, base, persistence) = new_persistence();

        persistence
            .persist_cells(vec![cell("EURUSD", 1, &[0, 1, 2, 3, 4])])
            .unwrap();

        assert_eq!(strategy_ids(&base, "EURUSD", 1), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn ids_continue_across_cells_in_one_batch() {
        let (_tmp, base, persistence) = new_persistence();

        persistence
            .persist_cells(vec![
                cell("EURUSD", 390, &[2001, 2002]),
                cell("USDJPY", 400, &[2001, 2002]),
            ])
            .unwrap();

        assert_eq!(strategy_ids(&base, "EURUSD", 390), vec![1, 2]);
        assert_eq!(strategy_ids(&base, "USDJPY", 400), vec![3, 4]);
    }

    #[test]
    fn later_persist_continues_from_global_max() {
        let (_tmp, base, persistence) = new_persistence();

        persistence
            .persist_cells(vec![cell("EURUSD", 390, &[2001, 2002])])
            .unwrap();
        persistence
            .persist_cells(vec![cell("EURUSD", 391, &[2001, 2002])])
            .unwrap();
        persistence
            .persist_cells(vec![cell("USDJPY", 400, &[2001, 2002])])
            .unwrap();

        assert_eq!(strategy_ids(&base, "USDJPY", 400), vec![5, 6]);
    }

    #[test]
    fn re_persisting_a_cell_keeps_original_ids() {
        let (_tmp, base, persistence) = new_persistence();

        persistence
            .persist_cells(vec![cell("EURUSD", 390, &[2001, 2002])])
            .unwrap();
        persistence
            .persist_cells(vec![cell("USDJPY", 400, &[2001, 2002])])
            .unwrap();
        persistence
            .persist_cells(vec![cell("EURUSD", 390, &[2001, 2002])])
            .unwrap();

        assert_eq!(strategy_ids(&base, "EURUSD", 390), vec![1, 2]);
        assert_eq!(strategy_ids(&base, "USDJPY", 400), vec![3, 4]);
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

    #[test]
    fn persist_cells_with_no_cells_writes_nothing() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let persistence = StrategyPersistence::new(root.join("strategy"), root.clone());

        persistence.persist_cells(Vec::new()).unwrap();

        assert!(std::fs::read_dir(&root).unwrap().next().is_none());
    }
}
