use std::collections::{HashMap, HashSet};
use std::time::Instant;

use anyhow::{bail, Context};
use log::{error, info};
use rayon::prelude::*;
use rustc_hash::FxHashMap;

use crate::application::cell_output::CellOutput;
use crate::application::source_data::connection_diagnostics;
use crate::application::strategy_runner::run_strategies;
use crate::application::strategy_runner_utils::get_unprocessed;
use crate::config::environment::{
    parquet_base_dir, parquet_root_dir, resolve_environment, DbConfig, Environment,
};
use crate::domain::indicator::{Indicator, IndicatorRepository};
use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};
use crate::infrastructure::pg_adapter::PgStore;
use crate::infrastructure::strategy_persistence::StrategyPersistence;

struct SourceIndex {
    indicators: Vec<Indicator>,
    indicator_ids: Vec<i32>,
    resampled_by_instrument: FxHashMap<String, Vec<ResampledPointD1>>,
    long_by_cell: FxHashMap<(String, i32), Vec<OperationPoint>>,
    short_by_cell: FxHashMap<(String, i32), Vec<OperationPoint>>,
}

impl SourceIndex {
    /// Loads every table once and indexes the rows in memory.
    fn load(env: Environment) -> anyhow::Result<Self> {
        info!("Querying DB...");
        let db_config = DbConfig::from_env(env);
        let conn_string = db_config.connection_string();

        // One store (connection) per table: the four loads below run
        // concurrently via nested rayon::join, and a sync postgres::Client
        // is single-connection. The single-cell `create` path needs only
        // one store for its sequential reads.
        let connect =
            || PgStore::new(&conn_string).with_context(|| connection_diagnostics(&db_config));
        let indicator_store = connect()?;
        let resampled_store = connect()?;
        let long_store = connect()?;
        let short_store = connect()?;

        let load_start = Instant::now();
        let (indicators, (resampled_by_instrument, (long_by_cell, short_by_cell))) = rayon::join(
            move || IndicatorRepository::get_all(&indicator_store),
            move || {
                rayon::join(
                    move || ResampledPointD1Repository::index_by_instrument(&resampled_store),
                    move || {
                        rayon::join(
                            move || LongOperationPointRepository::index_by_cell(&long_store),
                            move || ShortOperationPointRepository::index_by_cell(&short_store),
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

        let indicator_ids = indicators.iter().map(|indicator| indicator.id).collect();

        Ok(Self {
            indicators,
            indicator_ids,
            resampled_by_instrument,
            long_by_cell,
            short_by_cell,
        })
    }

    /// The `(instrument, mms_id)` cells that have operation points.
    fn cells(&self) -> Vec<(String, i32)> {
        let mut cells: Vec<(String, i32)> = self
            .long_by_cell
            .keys()
            .chain(self.short_by_cell.keys())
            .cloned()
            .collect();
        cells.sort_unstable();
        cells.dedup();
        cells
    }
}

enum CellOutcome {
    Computed(CellOutput),
    AlreadyProcessed,
    Failed { instrument: String, mms_id: i32 },
}

fn compute_cell(
    source: &SourceIndex,
    cell: &(String, i32),
    processed_ids_per_cell: &HashMap<(String, i32), HashSet<i32>>,
) -> CellOutcome {
    let (instrument, mms_id) = cell;

    let resampled_points: &[ResampledPointD1] = source
        .resampled_by_instrument
        .get(instrument)
        .map_or(&[], Vec::as_slice);
    let long_operation_points: &[OperationPoint] =
        source.long_by_cell.get(cell).map_or(&[], Vec::as_slice);
    let short_operation_points: &[OperationPoint] =
        source.short_by_cell.get(cell).map_or(&[], Vec::as_slice);

    let empty_processed_ids: HashSet<i32> = HashSet::new();
    let processed_ids = processed_ids_per_cell
        .get(cell)
        .unwrap_or(&empty_processed_ids);
    let unprocessed_indicator_ids = get_unprocessed(&source.indicator_ids, processed_ids);
    if unprocessed_indicator_ids.is_empty() {
        return CellOutcome::AlreadyProcessed;
    }

    let strategies = match run_strategies(
        &source.indicators,
        resampled_points,
        long_operation_points,
        short_operation_points,
        *mms_id,
        &unprocessed_indicator_ids,
    ) {
        Ok(strategies) => strategies,
        Err(e) => {
            error!("{instrument} (mms_id={mms_id}): {e:#}");
            return CellOutcome::Failed {
                instrument: instrument.clone(),
                mms_id: *mms_id,
            };
        }
    };

    CellOutcome::Computed(CellOutput {
        instrument: instrument.clone(),
        mms_id: *mms_id,
        strategies,
        computed_indicator_ids: unprocessed_indicator_ids,
    })
}

struct BatchSummary {
    computed: usize,
    skipped: usize,
    failed_cells: Vec<(String, i32)>,
}

impl BatchSummary {
    const fn failed(&self) -> usize {
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
            CellOutcome::Failed { instrument, mms_id } => failed_cells.push((instrument, mms_id)),
        }
    }

    let summary = BatchSummary {
        computed: computed_cells.len(),
        skipped,
        failed_cells,
    };
    (computed_cells, summary)
}

fn format_failed_cells(failed_cells: &[(String, i32)]) -> String {
    failed_cells
        .iter()
        .map(|(instrument, mms_id)| format!("{instrument} (mms_id={mms_id})"))
        .collect::<Vec<String>>()
        .join(", ")
}

// Exit-policy predicate: every cell that attempted work failed and none
// computed, the signature of a systemic outage (the DB going down mid-run)
// rather than isolated per-cell failures.
const fn is_total_failure(summary: &BatchSummary) -> bool {
    summary.failed() > 0 && summary.computed == 0
}

pub fn execute() -> anyhow::Result<()> {
    let command_start = Instant::now();
    // 1. Load and index the source data, and read the processed manifest, each once.
    let env = resolve_environment()?;
    let source = SourceIndex::load(env)?;
    let cells = source.cells();
    let persistence = StrategyPersistence::new(parquet_base_dir(env), parquet_root_dir(env));
    let processed_ids_per_cell = persistence
        .read_processed_ids_per_cell()
        .context("failed to read processed manifest")?;

    info!("{} cells to process", cells.len());

    // 2. Compute strategies in parallel.
    info!("Computing strategies...");
    let compute_start = Instant::now();
    let outcomes: Vec<CellOutcome> = cells
        .par_iter()
        .map(|cell| compute_cell(&source, cell, &processed_ids_per_cell))
        .collect();
    let (computed, summary) = split_outcomes(outcomes);
    info!(
        "Compute done: {} computed, {} skipped (already processed), {} failed in {:?}",
        summary.computed,
        summary.skipped,
        summary.failed(),
        compute_start.elapsed()
    );

    // 3. Persist the computed cells (assign IDs, write partitions in parallel, commit manifest).
    let persist_start = Instant::now();
    persistence
        .persist_cells(computed)
        .with_context(|| format!("failed to persist {} computed cells", summary.computed))?;
    info!(
        "Persist done: {} cells in {:?}",
        summary.computed,
        persist_start.elapsed()
    );

    info!(
        "Done: {} computed, {} skipped (already processed), {} failed. Total command time: {:?}",
        summary.computed,
        summary.skipped,
        summary.failed(),
        command_start.elapsed()
    );

    if summary.failed() > 0 {
        let failed_cells = format_failed_cells(&summary.failed_cells);
        error!(
            "{} of {} cells failed and will be retried on the next run: {}",
            summary.failed(),
            cells.len(),
            failed_cells
        );
    }

    if is_total_failure(&summary) {
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

    fn computed_outcome(instrument: &str, mms_id: i32) -> CellOutcome {
        CellOutcome::Computed(CellOutput {
            instrument: instrument.to_string(),
            mms_id,
            strategies: Vec::new(),
            computed_indicator_ids: Vec::new(),
        })
    }

    fn failed_outcome(instrument: &str, mms_id: i32) -> CellOutcome {
        CellOutcome::Failed {
            instrument: instrument.to_string(),
            mms_id,
        }
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
            vec![("EURUSD".to_string(), 7), ("USDJPY".to_string(), 3),]
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
        let cells: Vec<(String, i32)> = computed
            .into_iter()
            .map(|cell_output| (cell_output.instrument, cell_output.mms_id))
            .collect();
        assert_eq!(
            cells,
            vec![("USDJPY".to_string(), 2), ("EURUSD".to_string(), 1)]
        );
    }

    #[test]
    fn format_failed_cells_matches_the_per_cell_error_shape() {
        let failed_cells = vec![("EURUSD".to_string(), 7), ("USDJPY".to_string(), 3)];
        assert_eq!(
            format_failed_cells(&failed_cells),
            "EURUSD (mms_id=7), USDJPY (mms_id=3)"
        );
    }

    #[test]
    fn format_failed_cells_with_no_failures_is_empty() {
        assert_eq!(format_failed_cells(&[]), "");
    }

    #[test]
    fn every_cell_failed_is_total_failure() {
        let outcomes = vec![failed_outcome("EURUSD", 1), failed_outcome("USDJPY", 2)];
        let (_, summary) = split_outcomes(outcomes);
        assert!(is_total_failure(&summary));
    }

    #[test]
    fn failed_attempts_without_computed_is_total_failure_despite_skips() {
        // Already-processed cells were never attempted; when every cell that
        // WAS attempted fails, the run made no progress: treat as systemic.
        let outcomes = vec![CellOutcome::AlreadyProcessed, failed_outcome("EURUSD", 1)];
        let (_, summary) = split_outcomes(outcomes);
        assert!(is_total_failure(&summary));
    }

    #[test]
    fn partial_failure_is_not_total_failure() {
        let outcomes = vec![computed_outcome("EURUSD", 1), failed_outcome("USDJPY", 2)];
        let (_, summary) = split_outcomes(outcomes);
        assert!(!is_total_failure(&summary));
    }

    #[test]
    fn all_already_processed_is_not_total_failure() {
        let outcomes = vec![CellOutcome::AlreadyProcessed];
        let (_, summary) = split_outcomes(outcomes);
        assert!(!is_total_failure(&summary));
    }
}
