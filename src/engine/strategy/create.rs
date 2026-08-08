use std::time::{SystemTime, UNIX_EPOCH};

use crate::application::strategy_runner::run_strategies;
use crate::config::environment::{parquet_base_dir, parquet_root_dir};
use crate::infrastructure::processed_manifest::ParquetProcessedManifest;
use crate::infrastructure::strategy_adapter::ParquetStrategyRepository;

use super::resolve_environment;

/// Runs the strategy pipeline for a single (instrument, mms_id) pair and
/// writes results to parquet + manifest immediately.
pub fn execute(money_management_strategy_id: i32, instrument: String) -> anyhow::Result<()> {
    let Some((strategies, unprocessed_ids)) =
        run_strategies(&instrument, money_management_strategy_id)?
    else {
        return Ok(());
    };

    let env = resolve_environment();

    let repo = ParquetStrategyRepository::new(parquet_base_dir(env));
    eprintln!("Writing {} strategies to parquet ...", strategies.len());
    repo.write(&instrument, money_management_strategy_id, &strategies)?;

    let manifest = ParquetProcessedManifest::new(parquet_root_dir(env));
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
