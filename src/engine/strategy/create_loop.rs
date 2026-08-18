use std::collections::{HashMap, HashSet};

use anyhow::Context;
use rayon::prelude::*;

use crate::application::cell_output::CellOutput;
use crate::application::source_data::fetch_source_data;
use crate::application::strategy_runner::run_strategies;
use crate::application::strategy_runner_utils::get_unprocessed;
use crate::config::environment::{parquet_base_dir, parquet_root_dir, DbConfig};
use crate::infrastructure::pg_adapter::{
    PgMoneyManagementStrategyAdapter, PgResampledPointAdapter,
};
use crate::infrastructure::strategy_persistence::StrategyPersistence;

use super::resolve_environment;

fn discover_cells() -> anyhow::Result<Vec<(String, i32)>> {
    let db_config = DbConfig::from_env(resolve_environment());
    let conn_string = db_config.connection_string();

    let resampled_adapter = PgResampledPointAdapter::new(&conn_string)
        .context("Failed to connect for resampled points")?;
    let instruments = resampled_adapter.get_distinct_instruments();

    let mms_adapter = PgMoneyManagementStrategyAdapter::new(&conn_string)
        .context("Failed to connect for money management strategies")?;
    let mms_ids = mms_adapter.get_distinct_ids();

    Ok(instruments
        .iter()
        .flat_map(|inst| mms_ids.iter().map(|mms| (inst.clone(), *mms)))
        .collect())
}

fn compute_cell(
    instrument: &str,
    mms_id: i32,
    processed_ids_per_cell: &HashMap<(String, i32), HashSet<i32>>,
) -> Option<CellOutput> {
    let data = match fetch_source_data(instrument, mms_id) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error {} (mms_id={}): {:?}", instrument, mms_id, e);
            return None;
        }
    };

    let processed_ids = processed_ids_per_cell
        .get(&(instrument.to_string(), mms_id))
        .cloned()
        .unwrap_or_default();

    let all_indicator_ids: Vec<i32> = data.indicators.iter().map(|i| i.id).collect();
    let processed_indicator_ids = get_unprocessed(&all_indicator_ids, &processed_ids);
    if processed_indicator_ids.is_empty() {
        return None;
    }

    let strategies = match run_strategies(data, &processed_indicator_ids) {
        Ok(strategies) => strategies,
        Err(e) => {
            eprintln!("Error {} (mms_id={}): {:?}", instrument, mms_id, e);
            return None;
        }
    };

    Some(CellOutput {
        instrument: instrument.to_string(),
        mms_id,
        strategies,
        processed_indicator_ids,
    })
}

pub fn execute() -> anyhow::Result<()> {
    // Step 1: Discover cells and load the processed manifest (read once).
    let cells = discover_cells()?;
    let persistence = StrategyPersistence::new(
        parquet_base_dir(resolve_environment()),
        parquet_root_dir(resolve_environment()),
    );
    let processed_ids_per_cell = persistence.read_processed_ids_per_cell()?;

    eprintln!("{} cells to process.", cells.len());

    // Step 2: Compute strategies in parallel.
    let computed: Vec<CellOutput> = cells
        .par_iter()
        .filter_map(|(instrument, mms_id)| {
            compute_cell(instrument, *mms_id, &processed_ids_per_cell)
        })
        .collect();

    let cell_count = computed.len();
    eprintln!("Compute done — {} cells. Persisting ...", cell_count);

    // Step 3: Persist (assign IDs, write partitions in parallel, commit manifest).
    persistence.persist_cells(computed)?;

    eprintln!("Done — {} cells persisted.", cell_count);
    Ok(())
}
