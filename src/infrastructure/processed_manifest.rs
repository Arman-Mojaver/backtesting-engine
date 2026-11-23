use std::fs;
use std::fs::File;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use arrow::array::{Int32Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Int32Type, Int64Type, Schema};
use arrow::record_batch::RecordBatch;
use log::{info, warn};
use parquet::arrow::ArrowWriter;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::domain::cell::Cell;
use crate::infrastructure::parquet_columns::{primitive_column, string_column};

static TEMP_FILE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Unique per call so concurrent writes never share a temp file, and in
/// the same directory as `path` so the rename stays on one filesystem.
fn temp_path_for(path: &Path) -> PathBuf {
    let seq = TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!("parquet.tmp-{}-{seq}", std::process::id()))
}

/// Fails rather than falling back to epoch 0 if the clock is before 1970.
pub fn unix_now_secs() -> anyhow::Result<i64> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is set before the Unix epoch")?
        .as_secs();
    i64::try_from(secs).with_context(|| format!("Unix timestamp {secs} does not fit in i64"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestEntry {
    pub cell: Cell,
    pub indicator_ids: Vec<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProcessedRow {
    instrument: String,
    mms_id: i32,
    indicator_id: i32,
    processed_at: i64,
}

/// Ledger of the (instrument, mms_id, indicator_id) triples already
/// computed, persisted as a single `processed.parquet` under `root_dir` so
/// reruns only compute what is missing.
#[derive(Debug)]
pub struct ParquetProcessedManifest {
    root_dir: PathBuf,
}

impl ParquetProcessedManifest {
    pub const fn new(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    fn path(&self) -> PathBuf {
        self.root_dir.join("processed.parquet")
    }

    fn read_all(&self) -> anyhow::Result<Vec<ProcessedRow>> {
        let path = self.path();
        match File::open(&path) {
            Ok(file) => read_parquet(file)
                .with_context(|| format!("failed to read manifest {}", path.display())),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(e).with_context(|| format!("failed to open manifest {}", path.display())),
        }
    }

    pub fn append_all(&self, entries: &[ManifestEntry], processed_at: i64) -> anyhow::Result<()> {
        let existing = self.read_all()?;

        info!(
            "Writing manifest: {} entries ({} total indicator IDs) ...",
            entries.len(),
            entries.iter().map(|e| e.indicator_ids.len()).sum::<usize>(),
        );

        let rows = merge_entries(existing, entries, processed_at);
        write_atomically(&self.path(), &rows)
    }

    pub fn read_processed_ids_per_cell(&self) -> anyhow::Result<FxHashMap<Cell, FxHashSet<i32>>> {
        let mut map: FxHashMap<Cell, FxHashSet<i32>> = FxHashMap::default();
        for row in self.read_all()? {
            map.entry(Cell::new(row.instrument, row.mms_id))
                .or_default()
                .insert(row.indicator_id);
        }
        Ok(map)
    }
}

fn merge_entries(
    mut rows: Vec<ProcessedRow>,
    entries: &[ManifestEntry],
    processed_at: i64,
) -> Vec<ProcessedRow> {
    let mut incoming: FxHashSet<(&str, i32, i32)> = FxHashSet::default();
    let mut fresh_rows: Vec<ProcessedRow> = Vec::new();
    for entry in entries {
        for &indicator_id in &entry.indicator_ids {
            if incoming.insert((&entry.cell.instrument, entry.cell.mms_id, indicator_id)) {
                fresh_rows.push(ProcessedRow {
                    instrument: entry.cell.instrument.clone(),
                    mms_id: entry.cell.mms_id,
                    indicator_id,
                    processed_at,
                });
            }
        }
    }
    rows.retain(|r| !incoming.contains(&(r.instrument.as_str(), r.mms_id, r.indicator_id)));
    rows.extend(fresh_rows);
    rows
}

fn write_atomically(path: &Path, rows: &[ProcessedRow]) -> anyhow::Result<()> {
    if let Some(root_dir) = path.parent() {
        fs::create_dir_all(root_dir).with_context(|| {
            format!("failed to create manifest directory {}", root_dir.display())
        })?;
    }
    let temp_path = temp_path_for(path);
    let result = write_parquet(&temp_path, rows).and_then(|()| {
        fs::rename(&temp_path, path).with_context(|| {
            format!(
                "failed to rename {} to {}",
                temp_path.display(),
                path.display()
            )
        })
    });
    if result.is_err() {
        remove_temp_file(&temp_path);
    }
    result
}

fn remove_temp_file(temp_path: &Path) {
    match fs::remove_file(temp_path) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => warn!(
            "failed to remove manifest temp file {}: {e}",
            temp_path.display()
        ),
    }
}

fn read_parquet(file: File) -> anyhow::Result<Vec<ProcessedRow>> {
    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
    let mut reader = builder.build()?;
    let mut rows: Vec<ProcessedRow> = Vec::new();
    while let Some(batch) = reader.next().transpose()? {
        rows.extend(decode_batch(&batch)?);
    }
    Ok(rows)
}

fn decode_batch(batch: &RecordBatch) -> anyhow::Result<Vec<ProcessedRow>> {
    let instruments = string_column(batch, "instrument")?;
    let mms_ids = primitive_column::<Int32Type>(batch, "mms_id")?;
    let indicator_ids = primitive_column::<Int32Type>(batch, "indicator_id")?;
    let processed_ats = primitive_column::<Int64Type>(batch, "processed_at")?;

    let n = batch.num_rows();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(ProcessedRow {
            instrument: instruments.value(i).to_string(),
            mms_id: mms_ids.value(i),
            indicator_id: indicator_ids.value(i),
            processed_at: processed_ats.value(i),
        });
    }
    Ok(out)
}

fn write_parquet(path: &Path, rows: &[ProcessedRow]) -> anyhow::Result<()> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("instrument", DataType::Utf8, false),
        Field::new("mms_id", DataType::Int32, false),
        Field::new("indicator_id", DataType::Int32, false),
        Field::new("processed_at", DataType::Int64, false),
    ]));

    let instruments = StringArray::from_iter_values(rows.iter().map(|r| r.instrument.as_str()));
    let mms_ids: Int32Array = rows.iter().map(|r| r.mms_id).collect();
    let indicator_ids: Int32Array = rows.iter().map(|r| r.indicator_id).collect();
    let processed_ats: Int64Array = rows.iter().map(|r| r.processed_at).collect();

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(instruments),
            Arc::new(mms_ids),
            Arc::new(indicator_ids),
            Arc::new(processed_ats),
        ],
    )?;

    let props = WriterProperties::builder()
        .set_compression(Compression::ZSTD(ZstdLevel::default()))
        .build();

    let file = File::create(path)
        .with_context(|| format!("failed to create manifest {}", path.display()))?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
    writer.write(&batch)?;
    writer
        .into_inner()?
        .sync_all()
        .with_context(|| format!("failed to sync manifest {}", path.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::ArrayRef;
    use tempfile::TempDir;

    const FIRST_RUN_AT: i64 = 1_700_000_000;
    const SECOND_RUN_AT: i64 = 1_700_000_001;

    fn entry(instrument: &str, mms_id: i32, indicator_ids: &[i32]) -> ManifestEntry {
        ManifestEntry {
            cell: Cell::new(instrument, mms_id),
            indicator_ids: indicator_ids.to_vec(),
        }
    }

    fn read_cell(
        manifest: &ParquetProcessedManifest,
        instrument: &str,
        mms_id: i32,
    ) -> anyhow::Result<Vec<ProcessedRow>> {
        Ok(manifest
            .read_all()?
            .into_iter()
            .filter(|r| r.instrument == instrument && r.mms_id == mms_id)
            .collect())
    }

    fn new_manifest() -> (TempDir, ParquetProcessedManifest) {
        let tmp = TempDir::new().expect("tempdir");
        let manifest = ParquetProcessedManifest::new(tmp.path().to_path_buf());
        (tmp, manifest)
    }

    #[test]
    fn unix_now_secs_is_after_2020() {
        let now = unix_now_secs().expect("clock");
        assert!(now > 1_600_000_000, "unexpected timestamp {now}");
    }

    #[test]
    fn write_and_read_back() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], FIRST_RUN_AT)
            .expect("append");

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].instrument, "EURUSD");
        assert_eq!(rows[0].mms_id, 390);
        assert_eq!(rows[0].indicator_id, 2001);
        assert_eq!(rows[0].processed_at, FIRST_RUN_AT);
        assert_eq!(rows[1].indicator_id, 2002);
    }

    #[test]
    fn append_all_creates_a_missing_root_dir() {
        let tmp = TempDir::new().expect("tempdir");
        let manifest = ParquetProcessedManifest::new(tmp.path().join("alpha"));

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001])], FIRST_RUN_AT)
            .expect("append");

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].indicator_id, 2001);
    }

    #[test]
    fn append_all_new_rows_preserves_existing() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], FIRST_RUN_AT)
            .expect("first append");
        manifest
            .append_all(&[entry("EURUSD", 390, &[2003])], SECOND_RUN_AT)
            .expect("second append");

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert_eq!(rows.len(), 3);

        let ids: Vec<i32> = rows.iter().map(|r| r.indicator_id).collect();
        assert!(ids.contains(&2001));
        assert!(ids.contains(&2002));
        assert!(ids.contains(&2003));
    }

    #[test]
    fn append_all_with_duplicate_indicator_ids_drops_old() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], FIRST_RUN_AT)
            .expect("first append");

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2003])], SECOND_RUN_AT)
            .expect("re-run append");

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert_eq!(rows.len(), 3);

        let row_2001: Vec<&ProcessedRow> = rows.iter().filter(|r| r.indicator_id == 2001).collect();
        assert_eq!(row_2001.len(), 1);
        assert_eq!(row_2001[0].processed_at, SECOND_RUN_AT);

        let row_2002: Vec<&ProcessedRow> = rows.iter().filter(|r| r.indicator_id == 2002).collect();
        assert_eq!(row_2002.len(), 1);
        assert_eq!(row_2002[0].processed_at, FIRST_RUN_AT);

        let row_2003: Vec<&ProcessedRow> = rows.iter().filter(|r| r.indicator_id == 2003).collect();
        assert_eq!(row_2003.len(), 1);
        assert_eq!(row_2003[0].processed_at, SECOND_RUN_AT);
    }

    #[test]
    fn append_all_across_instruments_does_not_interfere() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], FIRST_RUN_AT)
            .expect("EURUSD");
        manifest
            .append_all(&[entry("USDJPY", 400, &[3001])], SECOND_RUN_AT)
            .expect("USDJPY");

        let eur_rows = read_cell(&manifest, "EURUSD", 390).expect("read EURUSD");
        let usd_rows = read_cell(&manifest, "USDJPY", 400).expect("read USDJPY");

        assert_eq!(eur_rows.len(), 2);
        assert_eq!(usd_rows.len(), 1);
        assert_eq!(usd_rows[0].indicator_id, 3001);
    }

    #[test]
    fn append_all_with_multiple_entries_records_each_cell() {
        let (_tmp, manifest) = new_manifest();
        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], FIRST_RUN_AT)
            .expect("first append");

        manifest
            .append_all(
                &[
                    entry("EURUSD", 390, &[2002, 2003]),
                    entry("USDJPY", 400, &[3001]),
                    entry("EURUSD", 390, &[2003, 2004]),
                ],
                SECOND_RUN_AT,
            )
            .expect("batch append");

        let eur: Vec<(i32, i64)> = read_cell(&manifest, "EURUSD", 390)
            .expect("read EURUSD")
            .iter()
            .map(|r| (r.indicator_id, r.processed_at))
            .collect();
        assert_eq!(
            eur,
            vec![
                (2001, FIRST_RUN_AT),
                (2002, SECOND_RUN_AT),
                (2003, SECOND_RUN_AT),
                (2004, SECOND_RUN_AT),
            ]
        );
        let usd = read_cell(&manifest, "USDJPY", 400).expect("read USDJPY");
        assert_eq!(usd.len(), 1);
        assert_eq!(usd[0].indicator_id, 3001);
    }

    #[test]
    fn empty_file_read_returns_empty() {
        let (_tmp, manifest) = new_manifest();

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert!(rows.is_empty());
    }

    #[test]
    fn read_processed_ids_per_cell_errors_when_manifest_cannot_be_opened() {
        let tmp = TempDir::new().expect("tempdir");
        let not_a_dir = tmp.path().join("root");
        fs::write(&not_a_dir, b"").expect("file");
        let manifest = ParquetProcessedManifest::new(not_a_dir);

        let err = manifest.read_processed_ids_per_cell().unwrap_err();
        assert!(
            format!("{err:#}").contains("failed to open manifest"),
            "{err:#}"
        );
    }

    #[test]
    fn multi_instrument_rows_read_filtered() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001])], FIRST_RUN_AT)
            .expect("EURUSD 390");
        manifest
            .append_all(&[entry("EURUSD", 391, &[2002])], FIRST_RUN_AT)
            .expect("EURUSD 391");
        manifest
            .append_all(&[entry("USDJPY", 390, &[3001])], FIRST_RUN_AT)
            .expect("USDJPY 390");

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].indicator_id, 2001);

        let rows = read_cell(&manifest, "EURUSD", 391).expect("read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].indicator_id, 2002);

        let rows = read_cell(&manifest, "USDJPY", 390).expect("read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].indicator_id, 3001);
    }

    #[test]
    fn append_all_with_empty_indicator_ids_noops() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001])], FIRST_RUN_AT)
            .expect("first append");

        manifest
            .append_all(&[entry("EURUSD", 390, &[])], SECOND_RUN_AT)
            .expect("empty append");

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].indicator_id, 2001);
        assert_eq!(rows[0].processed_at, FIRST_RUN_AT);
    }

    #[test]
    fn atomic_write_cleans_up_temp_file() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], FIRST_RUN_AT)
            .expect("first append");
        manifest
            .append_all(&[entry("EURUSD", 390, &[2003])], SECOND_RUN_AT)
            .expect("second append");

        let path = manifest.path();
        assert!(path.exists(), "processed.parquet should exist");

        let entries: Vec<String> = fs::read_dir(path.parent().expect("parent"))
            .expect("read_dir")
            .map(|e| e.expect("entry").file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(entries, vec!["processed.parquet"]);
    }

    #[test]
    fn write_atomically_removes_temp_file_when_rename_fails() {
        let tmp = TempDir::new().expect("tempdir");
        let target = tmp.path().join("processed.parquet");
        fs::create_dir(&target).expect("target dir");
        fs::write(target.join("occupant"), b"").expect("occupant");
        let rows = [ProcessedRow {
            instrument: "EURUSD".to_string(),
            mms_id: 390,
            indicator_id: 2001,
            processed_at: FIRST_RUN_AT,
        }];

        let result = write_atomically(&target, &rows);

        assert!(
            result.is_err(),
            "rename onto a non-empty directory must fail"
        );
        let entries: Vec<String> = fs::read_dir(tmp.path())
            .expect("read_dir")
            .map(|e| e.expect("entry").file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(entries, vec!["processed.parquet"]);
    }

    #[test]
    fn read_processed_ids_per_cell_returns_every_cell() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], FIRST_RUN_AT)
            .expect("EURUSD 390");
        manifest
            .append_all(&[entry("EURUSD", 391, &[2002])], FIRST_RUN_AT)
            .expect("EURUSD 391");
        manifest
            .append_all(&[entry("USDJPY", 390, &[3001])], FIRST_RUN_AT)
            .expect("USDJPY 390");

        let map = manifest.read_processed_ids_per_cell().expect("map");

        assert_eq!(map.len(), 3);
        assert_eq!(
            map.get(&Cell::new("EURUSD", 390)),
            Some(&FxHashSet::from_iter([2001, 2002]))
        );
        assert_eq!(
            map.get(&Cell::new("EURUSD", 391)),
            Some(&FxHashSet::from_iter([2002]))
        );
        assert_eq!(
            map.get(&Cell::new("USDJPY", 390)),
            Some(&FxHashSet::from_iter([3001]))
        );
    }

    fn utf8(values: &[&str]) -> ArrayRef {
        Arc::new(StringArray::from(values.to_vec()))
    }

    fn int32(values: &[i32]) -> ArrayRef {
        Arc::new(Int32Array::from(values.to_vec()))
    }

    fn int64(values: &[i64]) -> ArrayRef {
        Arc::new(Int64Array::from(values.to_vec()))
    }

    fn write_raw_manifest(manifest: &ParquetProcessedManifest, columns: Vec<(&str, ArrayRef)>) {
        let batch = RecordBatch::try_from_iter(columns).expect("batch");
        let file = File::create(manifest.path()).expect("create");
        let mut writer = ArrowWriter::try_new(file, batch.schema(), None).expect("writer");
        writer.write(&batch).expect("write");
        writer.close().expect("close");
    }

    #[test]
    fn read_processed_ids_per_cell_errors_on_missing_column() {
        let (_tmp, manifest) = new_manifest();
        write_raw_manifest(
            &manifest,
            vec![
                ("instrument", utf8(&["EURUSD"])),
                ("mms_id", int32(&[390])),
                ("processed_at", int64(&[FIRST_RUN_AT])),
            ],
        );

        let err = manifest.read_processed_ids_per_cell().unwrap_err();
        assert!(
            format!("{err:#}").contains("missing column indicator_id"),
            "{err:#}"
        );
    }

    #[test]
    fn read_processed_ids_per_cell_errors_on_wrong_column_type() {
        let (_tmp, manifest) = new_manifest();
        write_raw_manifest(
            &manifest,
            vec![
                ("instrument", utf8(&["EURUSD"])),
                ("mms_id", int64(&[390])),
                ("indicator_id", int32(&[2001])),
                ("processed_at", int64(&[FIRST_RUN_AT])),
            ],
        );

        let err = manifest.read_processed_ids_per_cell().unwrap_err();
        assert!(
            format!("{err:#}").contains("column mms_id is not Int32"),
            "{err:#}"
        );
    }

    #[test]
    fn read_processed_ids_per_cell_on_empty_manifest_is_empty() {
        let (_tmp, manifest) = new_manifest();

        let map = manifest.read_processed_ids_per_cell().expect("map");
        assert!(map.is_empty());
    }
}
