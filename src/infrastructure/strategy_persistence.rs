use std::path::PathBuf;

use anyhow::Context;
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::application::cell_output::CellOutput;
use crate::domain::cell::Cell;
use crate::infrastructure::processed_manifest::{
    ManifestEntry, ParquetProcessedManifest, unix_now_secs,
};
use crate::infrastructure::strategy_adapter::{ParquetStrategyRepository, StrategyRow};

const STRATEGY_DIR_NAME: &str = "strategy";

#[derive(Debug)]
pub struct StrategyPersistence {
    strategies: ParquetStrategyRepository,
    manifest: ParquetProcessedManifest,
}

impl StrategyPersistence {
    pub fn new(parquet_root: PathBuf) -> Self {
        Self {
            strategies: ParquetStrategyRepository::new(parquet_root.join(STRATEGY_DIR_NAME)),
            manifest: ParquetProcessedManifest::new(parquet_root),
        }
    }

    pub fn read_processed_ids_per_cell(&self) -> anyhow::Result<FxHashMap<Cell, FxHashSet<i32>>> {
        self.manifest.read_processed_ids_per_cell()
    }

    pub fn persist_cells(&self, cells: Vec<CellOutput>) -> anyhow::Result<()> {
        if cells.is_empty() {
            return Ok(());
        }

        // strategy_id is persistence-assigned: new ids start from the global max
        // across all partitions, so they stay unique across the whole tree.
        // Intentional: no inter-process lock. bte commands are not expected to
        // run concurrently.
        let mut next_id = self.strategies.max_strategy_id()?.saturating_add(1);
        let mut partition_writes: Vec<(Cell, Vec<StrategyRow>)> = Vec::with_capacity(cells.len());
        let mut manifest_entries: Vec<ManifestEntry> = Vec::with_capacity(cells.len());
        for cell_output in cells {
            let mut rows = Vec::with_capacity(cell_output.strategies.len());
            for strategy in cell_output.strategies {
                rows.push(StrategyRow::from_strategy(strategy, next_id));
                next_id = next_id.saturating_add(1);
            }
            manifest_entries.push(ManifestEntry {
                cell: cell_output.cell.clone(),
                indicator_ids: cell_output.computed_indicator_ids,
            });
            partition_writes.push((cell_output.cell, rows));
        }

        partition_writes
            .into_par_iter()
            .try_for_each(|(cell, rows)| {
                self.strategies
                    .write_rows(&cell, rows)
                    .with_context(|| format!("failed to write partition {cell}"))
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
            cell: Cell::new(instrument, mms_id),
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
        let persistence = StrategyPersistence::new(root);
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
        let persistence = StrategyPersistence::new(root);

        let cells = vec![
            CellOutput {
                cell: Cell::new("EURUSD", 390),
                strategies: vec![strategy("EURUSD", 2001, 390), strategy("EURUSD", 2002, 390)],
                computed_indicator_ids: vec![2001, 2002, 2003],
            },
            CellOutput {
                cell: Cell::new("USDJPY", 400),
                strategies: vec![strategy("USDJPY", 3001, 400)],
                computed_indicator_ids: vec![3001],
            },
        ];

        persistence.persist_cells(cells).unwrap();

        // Both partitions written.
        assert!(
            base.join("instrument=EURUSD/money_management_strategy_id=390/strategies.parquet")
                .exists()
        );
        assert!(
            base.join("instrument=USDJPY/money_management_strategy_id=400/strategies.parquet")
                .exists()
        );

        // Manifest records the full processed set, including 2003 (which had no strategy).
        let processed = persistence.read_processed_ids_per_cell().unwrap();
        let eur = processed.get(&Cell::new("EURUSD", 390)).unwrap();
        assert_eq!(eur.len(), 3);
        assert!(eur.contains(&2001));
        assert!(eur.contains(&2002));
        assert!(eur.contains(&2003));

        let usd = processed.get(&Cell::new("USDJPY", 400)).unwrap();
        assert_eq!(usd.len(), 1);
        assert!(usd.contains(&3001));
    }

    #[test]
    fn manifest_is_not_written_when_a_partition_write_fails() {
        let (_tmp, base, persistence) = new_persistence();
        std::fs::create_dir_all(&base).unwrap();
        std::fs::write(base.join("instrument=EURUSD"), b"").unwrap();

        let result = persistence.persist_cells(vec![
            cell("EURUSD", 390, &[2001]),
            cell("USDJPY", 400, &[3001]),
        ]);

        assert!(result.is_err());
        let processed = persistence.read_processed_ids_per_cell().unwrap();
        assert!(processed.is_empty());
    }

    #[test]
    fn persist_cells_with_no_cells_writes_nothing() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let persistence = StrategyPersistence::new(root.clone());

        persistence.persist_cells(Vec::new()).unwrap();

        assert!(std::fs::read_dir(&root).unwrap().next().is_none());
    }
}
