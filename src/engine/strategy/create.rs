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

pub fn execute(money_management_strategy_id: i32, instrument: &str) -> anyhow::Result<()> {
    let command_start = Instant::now();
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
    if unprocessed_ids.is_empty() {
        info!(
            "All {} indicators already processed for {} (mms_id={}). Nothing to do.",
            all_indicator_ids.len(),
            instrument,
            money_management_strategy_id,
        );
        info!("Total command time: {:?}", command_start.elapsed());
        return Ok(());
    }

    info!(
        "{} indicators total, {} unprocessed for {} (mms_id={})",
        all_indicator_ids.len(),
        unprocessed_ids.len(),
        instrument,
        money_management_strategy_id,
    );

    info!(
        "Running strategies for {} unprocessed indicators (mms_id={}) ...",
        unprocessed_ids.len(),
        money_management_strategy_id
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
    info!(
        "run_strategies done in {:?}: {} strategies",
        run_start.elapsed(),
        strategies.len()
    );

    let strategy_count = strategies.len();
    let indicator_count = unprocessed_ids.len();
    info!(
        "Persisting {strategy_count} strategies and {indicator_count} processed indicator IDs ..."
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
        "Done. {strategy_count} strategies for {instrument} (mms_id={money_management_strategy_id}). {indicator_count} indicators marked as processed.",
    );
    info!("Total command time: {:?}", command_start.elapsed());

    Ok(())
}
