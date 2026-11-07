use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use arrow::array::{
    ArrayRef, AsArray, Int32Array, Int64Array, PrimitiveArray, StringArray, StringBuilder,
};
use arrow::datatypes::{ArrowPrimitiveType, DataType, Field, Int32Type, Int64Type, Schema};
use arrow::record_batch::RecordBatch;
use log::info;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;

/// Per-call temp-file suffix: concurrent manifest writes never share
/// a temp file, so their contents cannot interleave.
static TEMP_FILE_SEQ: AtomicU64 = AtomicU64::new(0);

/// Temp path for an atomic write of `path`: unique per call (process
/// id plus a counter) and in the same directory, so the rename stays
/// on one filesystem.
fn temp_path_for(path: &Path) -> PathBuf {
    let seq = TEMP_FILE_SEQ.fetch_add(1, Ordering::Relaxed);
    path.with_extension(format!("parquet.tmp-{}-{seq}", std::process::id()))
}

/// Current wall-clock time as Unix seconds, for stamping `processed_at`.
/// Fails rather than falling back to epoch 0 if the clock is before 1970.
pub(crate) fn unix_now_secs() -> anyhow::Result<i64> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock is set before the Unix epoch")?
        .as_secs();
    i64::try_from(secs).with_context(|| format!("Unix timestamp {secs} does not fit in i64"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessedRow {
    pub instrument: String,
    pub mms_id: i32,
    pub indicator_id: i32,
    pub processed_at: i64,
}

/// Ledger of the (instrument, mms_id, indicator_id) triples already
/// computed, persisted as a single `processed.parquet` under `root_dir` so
/// reruns only compute what is missing.
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
        if !path.exists() {
            return Ok(Vec::new());
        }
        read_parquet(&path)
    }

    pub fn append_all(
        &self,
        entries: &[(String, i32, Vec<i32>)],
        processed_at: i64,
    ) -> anyhow::Result<()> {
        let path = self.path();
        let mut rows = self.read_all()?;

        info!(
            "Writing manifest: {} entries ({} total indicator IDs) ...",
            entries.len(),
            entries.iter().map(|(_, _, ids)| ids.len()).sum::<usize>(),
        );

        for (instrument, mms_id, indicator_ids) in entries {
            let indicator_set: HashSet<i32> = indicator_ids.iter().copied().collect();
            rows.retain(|r| {
                !(r.instrument == *instrument
                    && r.mms_id == *mms_id
                    && indicator_set.contains(&r.indicator_id))
            });

            for &indicator_id in indicator_ids {
                rows.push(ProcessedRow {
                    instrument: instrument.clone(),
                    mms_id: *mms_id,
                    indicator_id,
                    processed_at,
                });
            }
        }

        let temp_path = temp_path_for(&path);
        write_parquet(&temp_path, &rows)?;
        fs::rename(&temp_path, &path).with_context(|| {
            format!(
                "failed to rename {} to {}",
                temp_path.display(),
                path.display()
            )
        })?;

        Ok(())
    }

    pub fn read_processed_ids_per_cell(
        &self,
    ) -> anyhow::Result<HashMap<(String, i32), HashSet<i32>>> {
        let mut map: HashMap<(String, i32), HashSet<i32>> = HashMap::new();
        for row in self.read_all()? {
            map.entry((row.instrument, row.mms_id))
                .or_default()
                .insert(row.indicator_id);
        }
        Ok(map)
    }
}

fn read_parquet(path: &Path) -> anyhow::Result<Vec<ProcessedRow>> {
    let file =
        File::open(path).with_context(|| format!("failed to open manifest {}", path.display()))?;
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

fn column<'a>(batch: &'a RecordBatch, name: &str) -> anyhow::Result<&'a ArrayRef> {
    batch
        .column_by_name(name)
        .with_context(|| format!("manifest is missing column {name}"))
}

fn string_column<'a>(batch: &'a RecordBatch, name: &str) -> anyhow::Result<&'a StringArray> {
    column(batch, name)?
        .as_string_opt::<i32>()
        .with_context(|| format!("manifest column {name} is not Utf8"))
}

fn primitive_column<'a, T: ArrowPrimitiveType>(
    batch: &'a RecordBatch,
    name: &str,
) -> anyhow::Result<&'a PrimitiveArray<T>> {
    column(batch, name)?
        .as_primitive_opt::<T>()
        .with_context(|| format!("manifest column {name} is not {}", T::DATA_TYPE))
}

fn write_parquet(path: &Path, rows: &[ProcessedRow]) -> anyhow::Result<()> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("instrument", DataType::Utf8, false),
        Field::new("mms_id", DataType::Int32, false),
        Field::new("indicator_id", DataType::Int32, false),
        Field::new("processed_at", DataType::Int64, false),
    ]));

    let mut instrument_builder = StringBuilder::new();
    for r in rows {
        instrument_builder.append_value(&r.instrument);
    }

    let mms_ids: Int32Array = rows.iter().map(|r| r.mms_id).collect();
    let indicator_ids: Int32Array = rows.iter().map(|r| r.indicator_id).collect();
    let processed_ats: Int64Array = rows.iter().map(|r| r.processed_at).collect();

    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(instrument_builder.finish()),
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
    use tempfile::TempDir;

    fn entry(instrument: &str, mms_id: i32, indicator_ids: &[i32]) -> (String, i32, Vec<i32>) {
        (instrument.to_string(), mms_id, indicator_ids.to_vec())
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
        let ts = 1700000000;

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], ts)
            .expect("append");

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].instrument, "EURUSD");
        assert_eq!(rows[0].mms_id, 390);
        assert_eq!(rows[0].indicator_id, 2001);
        assert_eq!(rows[0].processed_at, ts);
        assert_eq!(rows[1].indicator_id, 2002);
    }

    #[test]
    fn append_all_new_rows_preserves_existing() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], 1700000000)
            .expect("first append");
        manifest
            .append_all(&[entry("EURUSD", 390, &[2003])], 1700000001)
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

        // First run: process indicators 2001, 2002
        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], 1700000000)
            .expect("first append");

        // Re-run with indicator 2001 + new 2003: old 2001 should be replaced
        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2003])], 1700000001)
            .expect("re-run append");

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert_eq!(rows.len(), 3);

        // 2001 should exist exactly once, with the new timestamp
        let row_2001: Vec<&ProcessedRow> = rows.iter().filter(|r| r.indicator_id == 2001).collect();
        assert_eq!(row_2001.len(), 1);
        assert_eq!(row_2001[0].processed_at, 1700000001);

        // 2002 should still be there with old timestamp
        let row_2002: Vec<&ProcessedRow> = rows.iter().filter(|r| r.indicator_id == 2002).collect();
        assert_eq!(row_2002.len(), 1);
        assert_eq!(row_2002[0].processed_at, 1700000000);

        // 2003 should exist with new timestamp
        let row_2003: Vec<&ProcessedRow> = rows.iter().filter(|r| r.indicator_id == 2003).collect();
        assert_eq!(row_2003.len(), 1);
        assert_eq!(row_2003[0].processed_at, 1700000001);
    }

    #[test]
    fn append_all_across_instruments_does_not_interfere() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], 1700000000)
            .expect("EURUSD");
        manifest
            .append_all(&[entry("USDJPY", 400, &[3001])], 1700000001)
            .expect("USDJPY");

        let eur_rows = read_cell(&manifest, "EURUSD", 390).expect("read EURUSD");
        let usd_rows = read_cell(&manifest, "USDJPY", 400).expect("read USDJPY");

        assert_eq!(eur_rows.len(), 2);
        assert_eq!(usd_rows.len(), 1);
        assert_eq!(usd_rows[0].indicator_id, 3001);
    }

    #[test]
    fn empty_file_read_returns_empty() {
        let (_tmp, manifest) = new_manifest();

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert!(rows.is_empty());
    }

    #[test]
    fn multi_instrument_rows_read_filtered() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001])], 1700000000)
            .expect("EURUSD 390");
        manifest
            .append_all(&[entry("EURUSD", 391, &[2002])], 1700000000)
            .expect("EURUSD 391");
        manifest
            .append_all(&[entry("USDJPY", 390, &[3001])], 1700000000)
            .expect("USDJPY 390");

        // Read only EURUSD/390
        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].indicator_id, 2001);

        // Read EURUSD/391
        let rows = read_cell(&manifest, "EURUSD", 391).expect("read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].indicator_id, 2002);

        // Read USDJPY/390
        let rows = read_cell(&manifest, "USDJPY", 390).expect("read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].indicator_id, 3001);
    }

    #[test]
    fn append_all_with_empty_indicator_ids_noops() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001])], 1700000000)
            .expect("first append");

        // Append with empty slice: nothing added, nothing removed
        manifest
            .append_all(&[entry("EURUSD", 390, &[])], 1700000001)
            .expect("empty append");

        let rows = read_cell(&manifest, "EURUSD", 390).expect("read");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].indicator_id, 2001);
        assert_eq!(rows[0].processed_at, 1700000000);
    }

    #[test]
    fn atomic_write_cleans_up_temp_file() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], 1700000000)
            .expect("first append");
        manifest
            .append_all(&[entry("EURUSD", 390, &[2003])], 1700000001)
            .expect("second append");

        // Verify the manifest file exists
        let path = manifest.path();
        assert!(path.exists(), "processed.parquet should exist");

        // Verify no temp file remains after the atomic rename: the temp
        // name is unique per call, so assert the directory holds only the
        // manifest itself.
        let entries: Vec<String> = fs::read_dir(path.parent().expect("parent"))
            .expect("read_dir")
            .map(|e| e.expect("entry").file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(entries, vec!["processed.parquet"]);
    }

    #[test]
    fn read_processed_ids_per_cell_returns_every_cell() {
        let (_tmp, manifest) = new_manifest();

        manifest
            .append_all(&[entry("EURUSD", 390, &[2001, 2002])], 1700000000)
            .expect("EURUSD 390");
        manifest
            .append_all(&[entry("EURUSD", 391, &[2002])], 1700000000)
            .expect("EURUSD 391");
        manifest
            .append_all(&[entry("USDJPY", 390, &[3001])], 1700000000)
            .expect("USDJPY 390");

        let map = manifest.read_processed_ids_per_cell().expect("map");

        // One entry per distinct (instrument, mms_id) cell, each carrying the
        // cell's full processed-indicator-id set, unfiltered by any external
        // cell universe, so manifest-only cells reach the report builder.
        assert_eq!(map.len(), 3);
        assert_eq!(
            map.get(&("EURUSD".to_string(), 390)),
            Some(&HashSet::from([2001, 2002]))
        );
        assert_eq!(
            map.get(&("EURUSD".to_string(), 391)),
            Some(&HashSet::from([2002]))
        );
        assert_eq!(
            map.get(&("USDJPY".to_string(), 390)),
            Some(&HashSet::from([3001]))
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
                ("processed_at", int64(&[1_700_000_000])),
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
                ("processed_at", int64(&[1_700_000_000])),
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
