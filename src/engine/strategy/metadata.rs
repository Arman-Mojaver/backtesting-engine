use std::collections::{HashMap, HashSet};

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
        .context("Failed to connect for operation points")?;

    let indicator_ids = indicator_adapter.get_all_indicator_ids()?;
    let instruments = resampled_adapter.get_distinct_instruments()?;
    let mms_ids = long_adapter.get_distinct_mms_ids()?;

    // Fetch manifest state
    let root_dir = parquet_root_dir(env);
    let manifest = ParquetProcessedManifest::new(root_dir);

    let mut processed_ids_per_cell: HashMap<(String, i32), HashSet<i32>> = HashMap::new();
    for instrument in &instruments {
        for mms_id in &mms_ids {
            let rows = manifest.read(instrument, *mms_id)?;
            let ids: HashSet<i32> = rows.iter().map(|r| r.indicator_id).collect();
            if !ids.is_empty() {
                processed_ids_per_cell.insert((instrument.clone(), *mms_id), ids);
            }
        }
    }

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
