use std::collections::HashSet;
use std::time::Instant;

use anyhow::Context;
use log::info;

use crate::application::cell_output::CellOutput;
use crate::application::source_data::fetch_source_data;
use crate::application::strategy_runner::run_strategies;
use crate::application::strategy_runner_utils::get_unprocessed;
use crate::config::environment::{parquet_base_dir, parquet_root_dir, resolve_environment};
use crate::infrastructure::strategy_persistence::StrategyPersistence;

pub fn execute(money_management_strategy_id: i32, instrument: &str) -> anyhow::Result<()> {
    let command_start = Instant::now();
    create_strategies(money_management_strategy_id, instrument)?;
    info!("Total command time: {:?}", command_start.elapsed());
    Ok(())
}

fn create_strategies(money_management_strategy_id: i32, instrument: &str) -> anyhow::Result<()> {
    let env = resolve_environment()?;
    let persistence = StrategyPersistence::new(parquet_base_dir(env), parquet_root_dir(env));
    info!("Fetching shared data for {instrument} (mms_id={money_management_strategy_id}) ...");
    let fetch_start = Instant::now();
    let data = fetch_source_data(instrument, money_management_strategy_id)?;
    info!("Source data fetched in {:?}", fetch_start.elapsed());

    let processed_ids =
        read_processed_indicator_ids(&persistence, instrument, money_management_strategy_id)?;
    let all_indicator_ids: Vec<i32> = data.indicators.iter().map(|i| i.id).collect();
    let unprocessed_ids = get_unprocessed(&all_indicator_ids, &processed_ids);
    let indicator_count = all_indicator_ids.len();
    let unprocessed_count = unprocessed_ids.len();
    if unprocessed_ids.is_empty() {
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
        "run_strategies done in {:?}: {strategy_count} strategies",
        run_start.elapsed()
    );

    info!(
        "Persisting {strategy_count} strategies and {unprocessed_count} processed indicator IDs ..."
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
    info!("Persist done in {:?}", persist_start.elapsed());

    info!(
        "Done. {strategy_count} strategies for {instrument} (mms_id={money_management_strategy_id}). {unprocessed_count} indicators marked as processed.",
    );

    Ok(())
}

fn read_processed_indicator_ids(
    persistence: &StrategyPersistence,
    instrument: &str,
    money_management_strategy_id: i32,
) -> anyhow::Result<HashSet<i32>> {
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
