use std::time::Instant;

use anyhow::{Context, bail};
use log::{error, info};
use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::application::cell_output::CellOutput;
use crate::application::signal_generator::signal_group_cache_stats;
use crate::application::strategy_runner::run_strategies;
use crate::config::environment::DbConfig;
use crate::domain::cell::Cell;
use crate::domain::indicator::{
    Indicator, IndicatorRepository, indicator_ids, unprocessed_indicators,
};
use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};
use crate::infrastructure::pg_adapter::PgStore;

#[derive(Debug)]
pub struct GroupedSourceData {
    indicators: Vec<Indicator>,
    resampled_by_instrument: FxHashMap<String, Vec<ResampledPointD1>>,
    long_by_cell: FxHashMap<Cell, Vec<OperationPoint>>,
    short_by_cell: FxHashMap<Cell, Vec<OperationPoint>>,
}

impl GroupedSourceData {
    pub fn load(db_config: &DbConfig) -> anyhow::Result<Self> {
        info!("Querying DB...");

        let connect = || PgStore::connect(db_config);
        let mut indicator_store = connect()?;
        let mut resampled_store = connect()?;
        let mut long_store = connect()?;
        let mut short_store = connect()?;

        let load_start = Instant::now();
        let (indicators, (resampled_by_instrument, (long_by_cell, short_by_cell))) = rayon::join(
            move || IndicatorRepository::get_all(&mut indicator_store),
            move || {
                rayon::join(
                    move || {
                        ResampledPointD1Repository::get_all_grouped_by_instrument(
                            &mut resampled_store,
                        )
                    },
                    move || {
                        rayon::join(
                            move || {
                                LongOperationPointRepository::get_all_grouped_by_cell(
                                    &mut long_store,
                                )
                            },
                            move || {
                                ShortOperationPointRepository::get_all_grouped_by_cell(
                                    &mut short_store,
                                )
                            },
                        )
                    },
                )
            },
        );
        let indicators = indicators.context("failed to load indicators")?;
        let resampled_by_instrument =
            resampled_by_instrument.context("failed to load resampled points")?;
        let long_by_cell = long_by_cell.context("failed to load long operation points")?;
        let short_by_cell = short_by_cell.context("failed to load short operation points")?;
        info!("Source data loaded in {:?}", load_start.elapsed());

        Ok(Self {
            indicators,
            resampled_by_instrument,
            long_by_cell,
            short_by_cell,
        })
    }

    pub fn cells(&self) -> Vec<Cell> {
        let mut cells: Vec<Cell> = self
            .long_by_cell
            .keys()
            .chain(self.short_by_cell.keys())
            .cloned()
            .collect();
        cells.sort_unstable();
        cells.dedup();
        cells
    }

    fn resampled_points(&self, instrument: &str) -> &[ResampledPointD1] {
        self.resampled_by_instrument
            .get(instrument)
            .map_or(&[], Vec::as_slice)
    }

    fn long_operation_points(&self, cell: &Cell) -> &[OperationPoint] {
        self.long_by_cell.get(cell).map_or(&[], Vec::as_slice)
    }

    fn short_operation_points(&self, cell: &Cell) -> &[OperationPoint] {
        self.short_by_cell.get(cell).map_or(&[], Vec::as_slice)
    }
}

enum CellOutcome {
    Computed(CellOutput),
    AlreadyProcessed,
    Failed(Cell),
}

fn compute_cell(
    source: &GroupedSourceData,
    cell: &Cell,
    processed_ids_per_cell: &FxHashMap<Cell, FxHashSet<i32>>,
) -> CellOutcome {
    let empty_processed_ids: FxHashSet<i32> = FxHashSet::default();
    let processed_ids = processed_ids_per_cell
        .get(cell)
        .unwrap_or(&empty_processed_ids);
    let unprocessed_indicators = unprocessed_indicators(&source.indicators, processed_ids);
    if unprocessed_indicators.is_empty() {
        return CellOutcome::AlreadyProcessed;
    }

    let strategies = match run_strategies(
        &unprocessed_indicators,
        source.resampled_points(&cell.instrument),
        source.long_operation_points(cell),
        source.short_operation_points(cell),
        cell.mms_id,
    ) {
        Ok(strategies) => strategies,
        Err(e) => {
            error!("{cell}: {e:#}");
            return CellOutcome::Failed(cell.clone());
        }
    };

    CellOutcome::Computed(CellOutput {
        cell: cell.clone(),
        strategies,
        computed_indicator_ids: indicator_ids(&unprocessed_indicators),
    })
}

#[derive(Debug)]
pub struct BatchSummary {
    pub computed: usize,
    pub skipped: usize,
    pub failed_cells: Vec<Cell>,
}

impl BatchSummary {
    pub const fn failed(&self) -> usize {
        self.failed_cells.len()
    }
}

fn split_outcomes(outcomes: Vec<CellOutcome>) -> (Vec<CellOutput>, BatchSummary) {
    let mut computed_cells = Vec::new();
    let mut skipped: usize = 0;
    let mut failed_cells = Vec::new();
    for outcome in outcomes {
        match outcome {
            CellOutcome::Computed(cell_output) => computed_cells.push(cell_output),
            CellOutcome::AlreadyProcessed => skipped = skipped.saturating_add(1),
            CellOutcome::Failed(cell) => failed_cells.push(cell),
        }
    }

    let summary = BatchSummary {
        computed: computed_cells.len(),
        skipped,
        failed_cells,
    };
    (computed_cells, summary)
}

pub fn compute_cells(
    source: &GroupedSourceData,
    cells: &[Cell],
    processed_ids_per_cell: &FxHashMap<Cell, FxHashSet<i32>>,
) -> (Vec<CellOutput>, BatchSummary) {
    info!("{} cells to process", cells.len());
    info!("Computing strategies...");
    let compute_start = Instant::now();
    let outcomes: Vec<CellOutcome> = cells
        .par_iter()
        .map(|cell| compute_cell(source, cell, processed_ids_per_cell))
        .collect();
    let (computed, summary) = split_outcomes(outcomes);
    info!(
        "Compute done: {} computed, {} skipped (already processed), {} failed in {:?}",
        summary.computed,
        summary.skipped,
        summary.failed(),
        compute_start.elapsed()
    );
    let cache = signal_group_cache_stats();
    info!(
        "Signal group cache: {} hits, {} misses, {} entries",
        cache.hits, cache.misses, cache.entries
    );
    (computed, summary)
}

// Every cell that attempted work failed and none computed: the signature of a
// systemic outage (the DB going down mid-run) rather than isolated per-cell
// failures, which are left for the next run to retry.
pub fn fail_on_total_failure(summary: &BatchSummary) -> anyhow::Result<()> {
    if summary.failed() > 0 && summary.computed == 0 {
        bail!(
            "total failure: {} failed, {} skipped, 0 computed, no progress; likely systemic",
            summary.failed(),
            summary.skipped
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures;

    fn computed_outcome(instrument: &str, mms_id: i32) -> CellOutcome {
        CellOutcome::Computed(CellOutput {
            cell: Cell::new(instrument, mms_id),
            strategies: Vec::new(),
            computed_indicator_ids: Vec::new(),
        })
    }

    fn failed_outcome(instrument: &str, mms_id: i32) -> CellOutcome {
        CellOutcome::Failed(Cell::new(instrument, mms_id))
    }

    #[test]
    fn empty_outcomes_produce_zero_counts() {
        let (_, summary) = split_outcomes(Vec::new());
        assert_eq!(summary.computed, 0);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.failed(), 0);
        assert!(summary.failed_cells.is_empty());
    }

    #[test]
    fn all_computed_counts_every_cell() {
        let outcomes = vec![
            computed_outcome("EURUSD", 1),
            computed_outcome("EURUSD", 2),
            computed_outcome("USDJPY", 1),
        ];
        let (_, summary) = split_outcomes(outcomes);
        assert_eq!(summary.computed, 3);
        assert_eq!(summary.skipped, 0);
        assert_eq!(summary.failed(), 0);
        assert!(summary.failed_cells.is_empty());
    }

    #[test]
    fn all_already_processed_counts_every_cell_as_skipped() {
        let outcomes = vec![CellOutcome::AlreadyProcessed, CellOutcome::AlreadyProcessed];
        let (_, summary) = split_outcomes(outcomes);
        assert_eq!(summary.computed, 0);
        assert_eq!(summary.skipped, 2);
        assert_eq!(summary.failed(), 0);
        assert!(summary.failed_cells.is_empty());
    }

    #[test]
    fn mixed_outcomes_count_each_variant() {
        let outcomes = vec![
            computed_outcome("EURUSD", 1),
            CellOutcome::AlreadyProcessed,
            failed_outcome("GBPUSD", 2),
            computed_outcome("USDJPY", 1),
            CellOutcome::AlreadyProcessed,
            failed_outcome("USDJPY", 3),
        ];
        let (_, summary) = split_outcomes(outcomes);
        assert_eq!(summary.computed, 2);
        assert_eq!(summary.skipped, 2);
        assert_eq!(summary.failed(), 2);
    }

    #[test]
    fn failed_cells_keep_identity_for_the_error_summary() {
        let outcomes = vec![
            failed_outcome("EURUSD", 7),
            computed_outcome("EURUSD", 1),
            failed_outcome("USDJPY", 3),
        ];
        let (_, summary) = split_outcomes(outcomes);
        assert_eq!(
            summary.failed_cells,
            vec![Cell::new("EURUSD", 7), Cell::new("USDJPY", 3)]
        );
    }

    #[test]
    fn computed_cells_are_returned_in_outcome_order() {
        let outcomes = vec![
            computed_outcome("USDJPY", 2),
            failed_outcome("GBPUSD", 1),
            CellOutcome::AlreadyProcessed,
            computed_outcome("EURUSD", 1),
        ];
        let (computed, _) = split_outcomes(outcomes);
        let cells: Vec<Cell> = computed
            .into_iter()
            .map(|cell_output| cell_output.cell)
            .collect();
        assert_eq!(cells, vec![Cell::new("USDJPY", 2), Cell::new("EURUSD", 1)]);
    }

    #[test]
    fn every_cell_failed_is_total_failure() {
        let outcomes = vec![failed_outcome("EURUSD", 1), failed_outcome("USDJPY", 2)];
        let (_, summary) = split_outcomes(outcomes);
        assert!(fail_on_total_failure(&summary).is_err());
    }

    #[test]
    fn failed_attempts_without_computed_is_total_failure_despite_skips() {
        // Already-processed cells were never attempted; when every cell that
        // WAS attempted fails, the run made no progress: treat as systemic.
        let outcomes = vec![CellOutcome::AlreadyProcessed, failed_outcome("EURUSD", 1)];
        let (_, summary) = split_outcomes(outcomes);
        assert!(fail_on_total_failure(&summary).is_err());
    }

    #[test]
    fn partial_failure_is_not_total_failure() {
        let outcomes = vec![computed_outcome("EURUSD", 1), failed_outcome("USDJPY", 2)];
        let (_, summary) = split_outcomes(outcomes);
        assert!(fail_on_total_failure(&summary).is_ok());
    }

    #[test]
    fn all_already_processed_is_not_total_failure() {
        let outcomes = vec![CellOutcome::AlreadyProcessed];
        let (_, summary) = split_outcomes(outcomes);
        assert!(fail_on_total_failure(&summary).is_ok());
    }

    fn eurusd_cell() -> Cell {
        Cell::new("EURUSD", 1)
    }

    fn grouped_source_data(
        indicators: Vec<Indicator>,
        resampled_points: Vec<ResampledPointD1>,
    ) -> GroupedSourceData {
        GroupedSourceData {
            indicators,
            resampled_by_instrument: FxHashMap::from_iter([(
                "EURUSD".to_string(),
                resampled_points,
            )]),
            long_by_cell: FxHashMap::from_iter([(eurusd_cell(), fixtures::op_points_default())]),
            short_by_cell: FxHashMap::from_iter([(eurusd_cell(), fixtures::op_points_default())]),
        }
    }

    fn processed(indicator_ids: &[i32]) -> FxHashMap<Cell, FxHashSet<i32>> {
        FxHashMap::from_iter([(eurusd_cell(), indicator_ids.iter().copied().collect())])
    }

    #[test]
    fn cells_are_sorted_union_of_long_and_short_without_duplicates() {
        let cell = |instrument: &str, mms_id| (Cell::new(instrument, mms_id), Vec::new());
        let source = GroupedSourceData {
            indicators: Vec::new(),
            resampled_by_instrument: FxHashMap::default(),
            long_by_cell: FxHashMap::from_iter([cell("USDJPY", 1), cell("EURUSD", 2)]),
            short_by_cell: FxHashMap::from_iter([cell("EURUSD", 2), cell("EURUSD", 1)]),
        };

        assert_eq!(
            source.cells(),
            vec![
                Cell::new("EURUSD", 1),
                Cell::new("EURUSD", 2),
                Cell::new("USDJPY", 1),
            ]
        );
    }

    #[test]
    fn compute_cell_computes_only_unprocessed_indicators() {
        let source = grouped_source_data(
            fixtures::indicators_default(),
            fixtures::resampled_points_default(),
        );

        let outcome = compute_cell(&source, &eurusd_cell(), &processed(&[1]));

        let CellOutcome::Computed(output) = outcome else {
            panic!("expected the cell to be computed");
        };
        assert_eq!(output.cell, Cell::new("EURUSD", 1));
        assert_eq!(output.computed_indicator_ids, vec![2]);
    }

    #[test]
    fn compute_cell_skips_cell_whose_indicators_are_all_processed() {
        let source = grouped_source_data(
            fixtures::indicators_default(),
            fixtures::resampled_points_default(),
        );

        let outcome = compute_cell(&source, &eurusd_cell(), &processed(&[1, 2]));

        assert!(matches!(outcome, CellOutcome::AlreadyProcessed));
    }

    #[test]
    fn compute_cell_reports_failed_cell_when_instrument_has_no_resampled_points() {
        let source = grouped_source_data(fixtures::indicators_default(), Vec::new());

        let outcome = compute_cell(&source, &eurusd_cell(), &FxHashMap::default());

        let CellOutcome::Failed(cell) = outcome else {
            panic!("expected the cell to fail");
        };
        assert_eq!(cell, eurusd_cell());
    }

    #[test]
    fn compute_cell_with_no_indicators_is_already_processed() {
        let source = grouped_source_data(Vec::new(), fixtures::resampled_points_default());

        let outcome = compute_cell(&source, &eurusd_cell(), &FxHashMap::default());

        assert!(matches!(outcome, CellOutcome::AlreadyProcessed));
    }
}
