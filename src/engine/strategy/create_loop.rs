use std::collections::{HashMap, HashSet};
use std::time::Instant;

use anyhow::Context;
use log::{error, info};
use rayon::prelude::*;
use rustc_hash::FxHashMap;

use crate::application::cell_output::CellOutput;
use crate::application::strategy_runner::run_strategies;
use crate::application::strategy_runner_utils::get_unprocessed;
use crate::config::environment::{parquet_base_dir, parquet_root_dir, DbConfig};
use crate::domain::indicator::{Indicator, IndicatorRepository};
use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};
use crate::infrastructure::pg_adapter::{
    PgIndicatorAdapter, PgLongOperationPointAdapter, PgResampledPointAdapter,
    PgShortOperationPointAdapter,
};
use crate::infrastructure::strategy_persistence::StrategyPersistence;

use super::resolve_environment;

/// The source data, loaded from the database and indexed for the parallel compute phase.
struct SourceIndex {
    indicators: Vec<Indicator>,
    indicator_ids: Vec<i32>,
    resampled_by_instrument: FxHashMap<String, Vec<ResampledPointD1>>,
    long_by_cell: FxHashMap<(String, i32), Vec<OperationPoint>>,
    short_by_cell: FxHashMap<(String, i32), Vec<OperationPoint>>,
}

impl SourceIndex {
    /// Loads every table once and indexes the rows in memory.
    fn load() -> anyhow::Result<Self> {
        let db_config = DbConfig::from_env(resolve_environment());
        let conn_string = db_config.connection_string();

        let indicator_adapter =
            PgIndicatorAdapter::new(&conn_string).context("Failed to connect for indicators")?;
        let resampled_adapter = PgResampledPointAdapter::new(&conn_string)
            .context("Failed to connect for resampled points")?;
        let long_adapter = PgLongOperationPointAdapter::new(&conn_string)
            .context("Failed to connect for long operation points")?;
        let short_adapter = PgShortOperationPointAdapter::new(&conn_string)
            .context("Failed to connect for short operation points")?;

        let load_start = Instant::now();
        // Each loader returns a Result; a failure in one table aborts the load
        // with the underlying error instead of panicking inside a rayon worker.
        let (indicators, (resampled_by_instrument, (long_by_cell, short_by_cell))) = rayon::join(
            move || indicator_adapter.get_all(),
            move || {
                rayon::join(
                    move || resampled_adapter.index_by_instrument(),
                    move || {
                        rayon::join(
                            move || long_adapter.index_by_cell(),
                            move || short_adapter.index_by_cell(),
                        )
                    },
                )
            },
        );
        let indicators = indicators?;
        let resampled_by_instrument = resampled_by_instrument?;
        let long_by_cell = long_by_cell?;
        let short_by_cell = short_by_cell?;
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

fn compute_cell(
    source: &SourceIndex,
    cell: &(String, i32),
    processed_ids_per_cell: &HashMap<(String, i32), HashSet<i32>>,
) -> Option<CellOutput> {
    let (instrument, mms_id) = cell;

    let resampled_points = source
        .resampled_by_instrument
        .get(instrument)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let long_operation_points = source
        .long_by_cell
        .get(cell)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let short_operation_points = source
        .short_by_cell
        .get(cell)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let processed_ids = processed_ids_per_cell
        .get(cell)
        .cloned()
        .unwrap_or_default();
    let processed_indicator_ids = get_unprocessed(&source.indicator_ids, &processed_ids);
    if processed_indicator_ids.is_empty() {
        return None;
    }

    let strategies = match run_strategies(
        &source.indicators,
        resampled_points,
        long_operation_points,
        short_operation_points,
        *mms_id,
        &processed_indicator_ids,
    ) {
        Ok(strategies) => strategies,
        Err(e) => {
            error!("Error {} (mms_id={}): {:?}", instrument, mms_id, e);
            return None;
        }
    };

    Some(CellOutput {
        instrument: instrument.clone(),
        mms_id: *mms_id,
        strategies,
        processed_indicator_ids,
    })
}

pub fn execute() -> anyhow::Result<()> {
    // 1. Load and index the source data, and read the processed manifest, each once.
    let source = SourceIndex::load()?;
    let cells = source.cells();
    let persistence = StrategyPersistence::new(
        parquet_base_dir(resolve_environment()),
        parquet_root_dir(resolve_environment()),
    );
    let processed_ids_per_cell = persistence.read_processed_ids_per_cell()?;

    info!("{} cells to process.", cells.len());

    // 2. Compute strategies in parallel.
    let computed: Vec<CellOutput> = cells
        .par_iter()
        .filter_map(|cell| compute_cell(&source, cell, &processed_ids_per_cell))
        .collect();

    let cell_count = computed.len();
    info!("Compute done — {} cells. Persisting ...", cell_count);

    // 3. Persist (assign IDs, write partitions in parallel, commit manifest).
    persistence.persist_cells(computed)?;

    info!("Done — {} cells persisted.", cell_count);
    Ok(())
}
