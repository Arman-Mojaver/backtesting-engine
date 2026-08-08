use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;

use crate::application::strategy_runner::run_strategies;
use crate::config::environment::{parquet_base_dir, parquet_root_dir, DbConfig};
use crate::domain::strategy::Strategy;
use crate::infrastructure::pg_adapter::{
    PgMoneyManagementStrategyAdapter, PgResampledPointAdapter,
};
use crate::infrastructure::processed_manifest::ParquetProcessedManifest;
use crate::infrastructure::strategy_adapter::ParquetStrategyRepository;

use super::resolve_environment;

struct StrategyBatch {
    instrument: String,
    mms_id: i32,
    strategies: Vec<Strategy>,
    unprocessed_ids: Vec<i32>,
}

/// Processes a slice of (instrument, mms_id) pairs sequentially.
/// Returns batches for pairs that had work to do (skips already-processed).
fn process_slice(slice: &[(String, i32)]) -> Vec<StrategyBatch> {
    let mut batches = Vec::new();
    for (instrument, mms_id) in slice {
        match run_strategies(instrument, *mms_id) {
            Ok(Some((strategies, unprocessed_ids))) => {
                batches.push(StrategyBatch {
                    instrument: instrument.clone(),
                    mms_id: *mms_id,
                    strategies,
                    unprocessed_ids,
                });
            }
            Ok(None) => {} // already processed
            Err(e) => {
                eprintln!("Error {} (mms_id={}): {:?}", instrument, mms_id, e);
            }
        }
    }
    batches
}

/// Writes a slice of batches to parquet using pre-assigned strategy IDs.
fn write_slice(slice: &[(i64, &StrategyBatch)]) {
    let repo = ParquetStrategyRepository::new(parquet_base_dir(resolve_environment()));
    for (start_id, batch) in slice {
        if let Err(e) = repo.write_with_start_id(
            &batch.instrument,
            batch.mms_id,
            &batch.strategies,
            *start_id,
        ) {
            eprintln!(
                "Write error {} (mms_id={}): {:?}",
                batch.instrument, batch.mms_id, e
            );
        }
    }
}

/// Runs the idempotent strategy pipeline for all (instrument, mms_id) pairs
/// using (CPUs - 1) workers for compute and write phases.
///
/// # Phases
///
/// 1. **Compute** — worker threads call `run_strategies` for their
///    slice of pairs. Results collected into `StrategyBatch`es.
///
/// 2. **ID assignment** — scans existing parquet files once for the global max
///    strategy ID, then assigns contiguous IDs.
///
/// 3. **Write** — worker threads write parquet files concurrently via
///    `write_with_start_id`. Each (instrument, mms_id) has its own file.
///
/// 4. **Manifest** — single atomic `append_all` for all processed triples.
///
/// # Redis Cache
///
/// Instrument-scoped Redis key means concurrent threads for the same instrument
/// may race on writes. Benign — lost entries recompute next run; manifest is
/// the source of truth.
pub fn execute() -> anyhow::Result<()> {
    let env = resolve_environment();
    let db_config = DbConfig::from_env(env);
    let conn_string = db_config.connection_string();

    let resampled_adapter = PgResampledPointAdapter::new(&conn_string)
        .context("Failed to connect for resampled points")?;
    let instruments = resampled_adapter.get_distinct_instruments();

    let mms_adapter = PgMoneyManagementStrategyAdapter::new(&conn_string)
        .context("Failed to connect for money management strategies")?;
    let mms_ids = mms_adapter.get_distinct_ids();

    let pairs: Vec<(String, i32)> = instruments
        .iter()
        .flat_map(|inst| mms_ids.iter().map(|mms| (inst.clone(), *mms)))
        .collect();

    let num_workers = (std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(8))
    .saturating_sub(1)
    .max(1);

    eprintln!(
        "{} instruments x {} MMS IDs = {} pairs, {} workers.",
        instruments.len(),
        mms_ids.len(),
        pairs.len(),
        num_workers,
    );

    // ==============================================================
    //  Phase 1: Parallel compute
    // ==============================================================
    let batches: Vec<StrategyBatch> = std::thread::scope(|s| {
        let mut handles = Vec::new();

        for slice in pairs.chunks(pairs.len().div_ceil(num_workers)) {
            let slice = slice.to_vec();
            handles.push(s.spawn(move || process_slice(&slice)));
        }

        let mut all = Vec::new();
        for handle in handles {
            all.extend(handle.join().unwrap());
        }
        all
    });

    eprintln!(
        "Compute done — {} batches. Assigning IDs ...",
        batches.len()
    );

    // ==============================================================
    //  Phase 2: Assign strategy IDs
    // ==============================================================
    let repo = ParquetStrategyRepository::new(parquet_base_dir(env));
    let mut next_id = repo.max_strategy_id()? + 1;

    let mut batches_with_ids: Vec<(i64, &StrategyBatch)> = Vec::with_capacity(batches.len());
    for batch in &batches {
        let id = next_id;
        next_id += batch.strategies.len() as i64;
        batches_with_ids.push((id, batch));
    }

    // ==============================================================
    //  Phase 3: Parallel writes
    // ==============================================================
    std::thread::scope(|s| {
        let mut handles = Vec::new();

        for slice in batches_with_ids.chunks(batches_with_ids.len().div_ceil(num_workers)) {
            let slice: Vec<_> = slice.to_vec();
            handles.push(s.spawn(move || write_slice(&slice)));
        }

        for handle in handles {
            handle.join().unwrap();
        }
    });

    // ==============================================================
    //  Phase 4: Single manifest write
    // ==============================================================
    let manifest = ParquetProcessedManifest::new(parquet_root_dir(env));
    let entries: Vec<(String, i32, Vec<i32>)> = batches
        .iter()
        .map(|b| (b.instrument.clone(), b.mms_id, b.unprocessed_ids.clone()))
        .collect();
    let processed_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    manifest.append_all(&entries, processed_at)?;

    eprintln!("Done — {} pairs written.", batches.len());
    Ok(())
}
