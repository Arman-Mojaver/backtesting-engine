use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::application::source_data::fetch_source_data;
use crate::application::strategy_runner::run_strategies;
use crate::application::strategy_runner_utils::get_unprocessed;
use crate::config::environment::{parquet_base_dir, parquet_root_dir};
use crate::infrastructure::processed_manifest::ParquetProcessedManifest;
use crate::infrastructure::strategy_adapter::ParquetStrategyRepository;

use super::resolve_environment;

pub fn execute(money_management_strategy_id: i32, instrument: String) -> anyhow::Result<()> {
    eprintln!(
        "Fetching shared data for {} (mms_id={}) ...",
        instrument, money_management_strategy_id
    );
    let data = fetch_source_data(&instrument, money_management_strategy_id)?;

    let manifest = ParquetProcessedManifest::new(parquet_root_dir(resolve_environment()));
    let processed_rows = manifest.read(&instrument, money_management_strategy_id)?;
    let processed_ids: HashSet<i32> = processed_rows.iter().map(|r| r.indicator_id).collect();

    let all_indicator_ids: Vec<i32> = data.indicators.iter().map(|i| i.id).collect();
    let unprocessed_ids = get_unprocessed(&all_indicator_ids, &processed_ids);
    if unprocessed_ids.is_empty() {
        eprintln!(
            "All {} indicators already processed for {} (mms_id={}). Nothing to do.",
            all_indicator_ids.len(),
            instrument,
            money_management_strategy_id,
        );
        return Ok(());
    }

    eprintln!(
        "{} indicators total, {} unprocessed for {} (mms_id={})",
        all_indicator_ids.len(),
        unprocessed_ids.len(),
        instrument,
        money_management_strategy_id,
    );

    let strategies = run_strategies(
        &data.indicators,
        &data.resampled_points,
        &data.long_operation_points,
        &data.short_operation_points,
        data.money_management_strategy_id,
        &unprocessed_ids,
    )?;

    let repo = ParquetStrategyRepository::new(parquet_base_dir(resolve_environment()));
    eprintln!("Writing {} strategies to parquet ...", strategies.len());
    repo.write(&instrument, money_management_strategy_id, &strategies)?;

    let processed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    eprintln!(
        "Appending {} unprocessed indicator IDs to manifest ...",
        unprocessed_ids.len(),
    );
    manifest.append(
        &instrument,
        money_management_strategy_id,
        &unprocessed_ids,
        processed_at,
    )?;

    eprintln!(
        "Done. {} strategies for {} (mms_id={}). {} indicators marked as processed.",
        strategies.len(),
        instrument,
        money_management_strategy_id,
        unprocessed_ids.len(),
    );

    Ok(())
}
