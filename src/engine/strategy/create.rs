use std::collections::HashSet;
use std::time::Instant;

use log::info;

use crate::application::source_data::fetch_source_data;
use crate::application::strategy_runner::run_strategies;
use crate::application::strategy_runner_utils::get_unprocessed;
use crate::config::environment::{parquet_base_dir, parquet_root_dir, resolve_environment};
use crate::infrastructure::processed_manifest::{unix_now_secs, ParquetProcessedManifest};
use crate::infrastructure::strategy_adapter::ParquetStrategyRepository;

pub fn execute(money_management_strategy_id: i32, instrument: &str) -> anyhow::Result<()> {
    let command_start = Instant::now();
    let env = resolve_environment()?;
    info!("Fetching shared data for {instrument} (mms_id={money_management_strategy_id}) ...");
    let fetch_start = Instant::now();
    let data = fetch_source_data(instrument, money_management_strategy_id)?;
    info!("Source data fetched in {:?}", fetch_start.elapsed());

    let manifest = ParquetProcessedManifest::new(parquet_root_dir(env));
    let processed_rows = manifest.read(instrument, money_management_strategy_id)?;
    let processed_ids: HashSet<i32> = processed_rows.iter().map(|r| r.indicator_id).collect();

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
    )?;
    info!(
        "run_strategies done in {:?}: {} strategies",
        run_start.elapsed(),
        strategies.len()
    );

    let repo = ParquetStrategyRepository::new(parquet_base_dir(env));
    info!("Writing {} strategies to parquet ...", strategies.len());
    let write_start = Instant::now();
    repo.write(instrument, money_management_strategy_id, &strategies)?;
    info!("Parquet write done in {:?}", write_start.elapsed());

    let processed_at = unix_now_secs()?;

    info!(
        "Appending {} unprocessed indicator IDs to manifest ...",
        unprocessed_ids.len(),
    );
    manifest.append(
        instrument,
        money_management_strategy_id,
        &unprocessed_ids,
        processed_at,
    )?;

    info!(
        "Done. {} strategies for {} (mms_id={}). {} indicators marked as processed.",
        strategies.len(),
        instrument,
        money_management_strategy_id,
        unprocessed_ids.len(),
    );
    info!("Total command time: {:?}", command_start.elapsed());

    Ok(())
}
