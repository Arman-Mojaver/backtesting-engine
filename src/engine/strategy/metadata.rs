use anyhow::Context;
use log::info;

use crate::application::metadata_report::{
    build_metadata_report, DbMetadataState, ManifestMetadataState,
};
use crate::config::environment::{parquet_root_dir, resolve_environment, DbConfig};
use crate::infrastructure::pg_adapter::{
    PgIndicatorAdapter, PgLongOperationPointAdapter, PgResampledPointAdapter,
};
use crate::infrastructure::processed_manifest::ParquetProcessedManifest;

pub fn execute() -> anyhow::Result<()> {
    let env = resolve_environment()?;

    let db_config = DbConfig::from_env(env);
    let conn_string = db_config.connection_string();

    // Fetch DB state
    info!("Querying DB metadata ...");

    let indicator_adapter =
        PgIndicatorAdapter::new(&conn_string).context("Failed to connect for indicators")?;
    let resampled_adapter = PgResampledPointAdapter::new(&conn_string)
        .context("Failed to connect for resampled points")?;
    let long_adapter = PgLongOperationPointAdapter::new(&conn_string)
        .context("Failed to connect for long operation points")?;

    let indicator_ids = indicator_adapter.get_all_indicator_ids()?;
    let instruments = resampled_adapter.get_distinct_instruments()?;
    // The long and short operation-point tables carry the same
    // money-management id set (written together by the upstream
    // Backtesting-4.0 project), so the long table alone is a complete
    // id universe — no need to also query the short table here.
    let mms_ids = long_adapter.get_distinct_mms_ids()?;

    // Fetch manifest state in a single decode: the one-pass map builder reads
    // the manifest parquet once and returns the processed-indicator-id set for
    // every cell in it — including cells with no DB presence (manifest-only
    // cells), which the report builder renders with zero DB indicators.
    let root_dir = parquet_root_dir(env);
    let manifest = ParquetProcessedManifest::new(root_dir);
    let processed_ids_per_cell = manifest.read_processed_ids_per_cell()?;

    let db_state = DbMetadataState {
        instruments,
        mms_ids,
        indicator_ids,
    };

    let manifest_state = ManifestMetadataState {
        processed_ids_per_cell,
    };

    let report = build_metadata_report(&db_state, &manifest_state);
    println!("{report}");

    Ok(())
}
