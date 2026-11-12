use std::time::Instant;

use anyhow::Context;
use log::info;
use rustc_hash::FxHashSet;

use crate::application::cell_output::CellOutput;
use crate::application::source_data::fetch_source_data;
use crate::application::strategy_runner::run_strategies;
use crate::application::strategy_runner_utils::{get_unprocessed, indicator_ids};
use crate::config::environment::{
    parquet_base_dir, parquet_root_dir, resolve_environment, DbConfig,
};
use crate::infrastructure::strategy_persistence::StrategyPersistence;

pub fn execute(money_management_strategy_id: i32, instrument: &str) -> anyhow::Result<()> {
    let command_start = Instant::now();
    create_strategies(money_management_strategy_id, instrument)?;
    info!("Total command time: {:?}", command_start.elapsed());
    Ok(())
}

fn create_strategies(money_management_strategy_id: i32, instrument: &str) -> anyhow::Result<()> {
    let env = resolve_environment()?;
    let db_config = DbConfig::from_env(env);
    let persistence = StrategyPersistence::new(parquet_base_dir(env), parquet_root_dir(env));
    info!("Fetching shared data for {instrument} (mms_id={money_management_strategy_id}) ...");
    let fetch_start = Instant::now();
    let data = fetch_source_data(&db_config, instrument, money_management_strategy_id)?;
    info!("Source data fetched in {:?}", fetch_start.elapsed());

    let processed_ids =
        read_processed_indicator_ids(&persistence, instrument, money_management_strategy_id)?;
    let unprocessed_indicators = get_unprocessed(&data.indicators, &processed_ids);
    let indicator_count = data.indicators.len();
    let unprocessed_ids = indicator_ids(&unprocessed_indicators);
    let unprocessed_count = unprocessed_ids.len();
    if unprocessed_indicators.is_empty() {
        info!(
            "All {indicator_count} indicators already processed for {instrument} (mms_id={money_management_strategy_id}). Nothing to do."
        );
        return Ok(());
    }

    info!(
        "Running strategies for {unprocessed_count} unprocessed of {indicator_count} indicators for {instrument} (mms_id={money_management_strategy_id}) ..."
    );
    let run_start = Instant::now();
    let strategies = run_strategies(
        &data.indicators,
        &data.resampled_points,
        &data.long_operation_points,
        &data.short_operation_points,
        money_management_strategy_id,
        &unprocessed_ids,
    )
    .with_context(|| {
        format!("failed to run strategies for {instrument} (mms_id={money_management_strategy_id})")
    })?;
    let strategy_count = strategies.len();
    info!(
        "Strategies computed in {:?}: {strategy_count} strategies",
        run_start.elapsed()
    );

    let persist_start = Instant::now();
    persistence.persist_cells(vec![CellOutput {
        instrument: instrument.to_string(),
        mms_id: money_management_strategy_id,
        strategies,
        computed_indicator_ids: unprocessed_ids,
    }])
    .with_context(|| {
        format!("failed to persist strategies for {instrument} (mms_id={money_management_strategy_id})")
    })?;
    info!(
        "Persist done: {strategy_count} strategies and {unprocessed_count} processed indicator IDs in {:?}",
        persist_start.elapsed()
    );

    info!(
        "Done. {strategy_count} strategies for {instrument} (mms_id={money_management_strategy_id}). {unprocessed_count} indicators marked as processed.",
    );

    Ok(())
}

fn read_processed_indicator_ids(
    persistence: &StrategyPersistence,
    instrument: &str,
    money_management_strategy_id: i32,
) -> anyhow::Result<FxHashSet<i32>> {
    let mut processed_ids_per_cell = persistence
        .read_processed_ids_per_cell()
        .with_context(|| {
            format!(
                "failed to read processed manifest for {instrument} (mms_id={money_management_strategy_id})"
            )
        })?;
    let cell = (instrument.to_string(), money_management_strategy_id);
    Ok(processed_ids_per_cell.remove(&cell).unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn new_persistence() -> (TempDir, StrategyPersistence) {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();
        let persistence = StrategyPersistence::new(root.join("strategy"), root);
        (tmp, persistence)
    }

    fn processed_cell(instrument: &str, mms_id: i32, indicator_ids: &[i32]) -> CellOutput {
        CellOutput {
            instrument: instrument.to_string(),
            mms_id,
            strategies: Vec::new(),
            computed_indicator_ids: indicator_ids.to_vec(),
        }
    }

    #[test]
    fn read_processed_indicator_ids_is_empty_before_any_run() {
        let (_tmp, persistence) = new_persistence();

        let processed = read_processed_indicator_ids(&persistence, "EURUSD", 390).unwrap();

        assert!(processed.is_empty());
    }

    #[test]
    fn read_processed_indicator_ids_returns_only_the_requested_cell() {
        let (_tmp, persistence) = new_persistence();
        persistence
            .persist_cells(vec![
                processed_cell("EURUSD", 390, &[2001, 2002]),
                processed_cell("EURUSD", 391, &[3001]),
                processed_cell("USDJPY", 390, &[4001]),
            ])
            .unwrap();

        let processed = read_processed_indicator_ids(&persistence, "EURUSD", 390).unwrap();

        assert_eq!(processed, FxHashSet::from_iter([2001, 2002]));
    }

    #[test]
    fn read_processed_indicator_ids_is_empty_for_an_unseen_cell() {
        let (_tmp, persistence) = new_persistence();
        persistence
            .persist_cells(vec![processed_cell("EURUSD", 390, &[2001])])
            .unwrap();

        let processed = read_processed_indicator_ids(&persistence, "GBPUSD", 390).unwrap();

        assert!(processed.is_empty());
    }
}
