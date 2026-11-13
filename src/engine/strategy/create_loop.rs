use std::time::Instant;

use anyhow::Context;
use itertools::Itertools;
use log::{error, info};

use crate::application::cell_output::CellOutput;
use crate::application::strategy_batch::{
    BatchSummary, SourceIndex, compute_cells, fail_on_total_failure,
};
use crate::config::environment::{DbConfig, parquet_root_dir, resolve_environment};
use crate::domain::cell::Cell;
use crate::infrastructure::strategy_persistence::StrategyPersistence;

fn format_failed_cells(failed_cells: &[Cell]) -> String {
    failed_cells.iter().join(", ")
}

pub fn execute() -> anyhow::Result<()> {
    let command_start = Instant::now();
    let env = resolve_environment()?;
    let source = SourceIndex::load(&DbConfig::from_env(env))?;
    let cells = source.cells();
    let persistence = StrategyPersistence::new(parquet_root_dir(env));
    let processed_ids_per_cell = persistence
        .read_processed_ids_per_cell()
        .context("failed to read processed manifest")?;

    let (computed, summary) = compute_cells(&source, &cells, &processed_ids_per_cell);
    persist_computed_cells(&persistence, computed)?;

    report_run(&summary, cells.len(), command_start);
    fail_on_total_failure(&summary)
}

fn persist_computed_cells(
    persistence: &StrategyPersistence,
    computed: Vec<CellOutput>,
) -> anyhow::Result<()> {
    let cell_count = computed.len();
    let persist_start = Instant::now();
    persistence
        .persist_cells(computed)
        .with_context(|| format!("failed to persist {cell_count} computed cells"))?;
    info!(
        "Persist done: {cell_count} cells in {:?}",
        persist_start.elapsed()
    );
    Ok(())
}

fn report_run(summary: &BatchSummary, total_cells: usize, command_start: Instant) {
    info!(
        "Done: {} computed, {} skipped (already processed), {} failed. Total command time: {:?}",
        summary.computed,
        summary.skipped,
        summary.failed(),
        command_start.elapsed()
    );

    if summary.failed() > 0 {
        error!(
            "{} of {total_cells} cells failed and will be retried on the next run: {}",
            summary.failed(),
            format_failed_cells(&summary.failed_cells)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_failed_cells_matches_the_per_cell_error_shape() {
        let failed_cells = vec![Cell::new("EURUSD", 7), Cell::new("USDJPY", 3)];
        assert_eq!(
            format_failed_cells(&failed_cells),
            "EURUSD (mms_id=7), USDJPY (mms_id=3)"
        );
    }

    #[test]
    fn format_failed_cells_with_no_failures_is_empty() {
        assert_eq!(format_failed_cells(&[]), "");
    }
}
