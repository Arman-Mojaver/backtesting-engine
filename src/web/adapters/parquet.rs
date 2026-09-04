//! Parquet stats behind `spawn_blocking` (the parquet/arrow ecosystem is sync).
//!
//! Reuses the engine's sync adapters as-is: the manifest via
//! [`ParquetProcessedManifest`], strategy outputs via parquet footer
//! metadata (row counts without reading data pages).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use log::warn;
use parquet::file::reader::{FileReader, SerializedFileReader};

use crate::infrastructure::processed_manifest::ParquetProcessedManifest;
use crate::web::info::ParquetScan;

/// File name the strategy repository writes per (instrument x mms) partition.
const STRATEGY_FILE_NAME: &str = "strategies.parquet";

/// Collects manifest + strategy-output stats for the parquet tree.
pub async fn collect(parquet_root: String) -> Result<ParquetScan> {
    tokio::task::spawn_blocking(move || scan(&parquet_root))
        .await
        .map_err(|error| anyhow::anyhow!("parquet scan task panicked: {error}"))?
}

/// Blocking scan of the parquet tree.
fn scan(parquet_root: &str) -> Result<ParquetScan> {
    let manifest = ParquetProcessedManifest::new(parquet_root.to_string());
    let processed_ids_per_cell = manifest
        .read_processed_ids_per_cell()
        .context("failed to read processed manifest")?;

    let files = parquet_files(Path::new(parquet_root))?;
    let parquet_bytes: u64 = files.iter().map(|(_, size)| size).sum();

    let mut strategy_partitions = 0usize;
    let mut total_strategies = 0i64;
    for (path, _) in &files {
        if path
            .file_name()
            .is_some_and(|name| name == STRATEGY_FILE_NAME)
        {
            strategy_partitions += 1;
            match parquet_row_count(path) {
                Ok(rows) => total_strategies += rows,
                Err(error) => warn!("skipping unreadable {}: {error}", path.display()),
            }
        }
    }

    Ok(ParquetScan {
        processed_ids_per_cell,
        strategy_partitions,
        total_strategies,
        parquet_bytes,
    })
}

/// Recursively collects all `.parquet` files under `dir` with their sizes.
fn parquet_files(dir: &Path) -> Result<Vec<(PathBuf, u64)>> {
    let mut files = Vec::new();
    visit_parquet_files(dir, &mut files)?;
    Ok(files)
}

fn visit_parquet_files(dir: &Path, files: &mut Vec<(PathBuf, u64)>) -> Result<()> {
    let entries =
        fs::read_dir(dir).with_context(|| format!("failed to read dir {}", dir.display()))?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            visit_parquet_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "parquet") {
            let size = entry.metadata()?.len();
            files.push((path, size));
        }
    }
    Ok(())
}

/// Row count from a parquet footer (reads metadata only, not data).
fn parquet_row_count(path: &Path) -> Result<i64> {
    let file = fs::File::open(path)?;
    let reader = SerializedFileReader::new(file)?;
    Ok(reader.metadata().file_metadata().num_rows())
}
