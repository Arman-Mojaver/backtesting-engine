use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use arrow::array::{
    AsArray, Decimal128Array, Int32Array, Int32Builder, Int64Array, ListArray, ListBuilder,
    PrimitiveArray,
};
use arrow::datatypes::{DataType, Decimal128Type, Field, Int32Type, Int64Type, Schema};
use arrow::record_batch::RecordBatch;
use log::debug;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;
use rustc_hash::FxHashSet;

use crate::domain::cast;
use crate::domain::strategy::Strategy;

#[derive(Debug, Clone)]
pub struct StrategyRow {
    strategy_id: i64,
    indicator_id: i32,
    annual_roi: f64,
    max_draw_down: f64,
    annual_operation_count: f64,
    long_operation_point_ids: Vec<i32>,
    short_operation_point_ids: Vec<i32>,
    money_management_strategy_id: i32,
}

impl StrategyRow {
    pub fn from_strategy(s: &Strategy, id: i64) -> Self {
        Self {
            strategy_id: id,
            indicator_id: s.indicator_id,
            annual_roi: s.annual_roi,
            max_draw_down: s.max_draw_down,
            annual_operation_count: s.annual_operation_count,
            long_operation_point_ids: s.long_operation_point_ids.clone(),
            short_operation_point_ids: s.short_operation_point_ids.clone(),
            money_management_strategy_id: s.money_management_strategy_id,
        }
    }
}

pub struct ParquetStrategyRepository {
    base_dir: PathBuf,
}

impl ParquetStrategyRepository {
    pub const fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn write_rows(
        &self,
        instrument: &str,
        mms_id: i32,
        rows: Vec<StrategyRow>,
    ) -> anyhow::Result<()> {
        let path = self.partition_path(instrument, mms_id)?;
        Self::write_partition(&path, rows)
    }

    fn partition_path(&self, instrument: &str, mms_id: i32) -> anyhow::Result<PathBuf> {
        let partition_dir = self
            .base_dir
            .join(format!("instrument={instrument}"))
            .join(format!("money_management_strategy_id={mms_id}"));
        fs::create_dir_all(&partition_dir)?;
        Ok(partition_dir.join("strategies.parquet"))
    }

    fn write_partition(path: &Path, new_rows: Vec<StrategyRow>) -> anyhow::Result<()> {
        let start = Instant::now();
        let existing_rows: Vec<StrategyRow> = if path.exists() {
            Self::read_rows(path)?
        } else {
            Vec::new()
        };

        // indicator_id is the partition's identity: incoming duplicates are
        // dropped, so re-writing a partition preserves the original rows and
        // their ids.
        let existing_ids: FxHashSet<i32> = existing_rows.iter().map(|r| r.indicator_id).collect();
        let fresh_rows: Vec<StrategyRow> = new_rows
            .into_iter()
            .filter(|row| !existing_ids.contains(&row.indicator_id))
            .collect();

        if fresh_rows.is_empty() {
            debug!(
                "parquet partition skipped ({}): 0 fresh rows ({} existing) in {:?}",
                path.display(),
                existing_rows.len(),
                start.elapsed()
            );
            return Ok(());
        }

        let existing_count = existing_rows.len();
        let fresh_count = fresh_rows.len();
        let mut combined = existing_rows;
        combined.extend(fresh_rows);

        let batch = build_record_batch(&combined)?;
        let schema = batch.schema();

        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(ZstdLevel::default()))
            .build();

        let file = File::create(path)?;
        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        debug!(
            "parquet partition written ({}): {} rows ({} existing + {} fresh) in {:?}",
            path.display(),
            combined.len(),
            existing_count,
            fresh_count,
            start.elapsed()
        );

        Ok(())
    }

    pub fn max_strategy_id(&self) -> anyhow::Result<i64> {
        let start = Instant::now();
        if !self.base_dir.exists() {
            return Ok(0);
        }
        let mut max_id: i64 = 0;
        let mut partitions_scanned: usize = 0;
        let mut dirs: Vec<PathBuf> = vec![self.base_dir.clone()];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.file_name().is_some_and(|n| n == "strategies.parquet") {
                    partitions_scanned = partitions_scanned.saturating_add(1);
                    let file = File::open(&path)?;
                    let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
                    let mut reader = builder.build()?;
                    while let Some(batch) = reader.next().transpose()? {
                        for id in batch.column(0).as_primitive::<Int64Type>().values() {
                            max_id = max_id.max(*id);
                        }
                    }
                }
            }
        }
        debug!(
            "parquet max strategy id scan: {} partition(s), max_id={} in {:?}",
            partitions_scanned,
            max_id,
            start.elapsed()
        );
        Ok(max_id)
    }

    fn read_rows(path: &Path) -> anyhow::Result<Vec<StrategyRow>> {
        let file = File::open(path)?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let mut reader = builder.build()?;

        let mut rows: Vec<StrategyRow> = Vec::new();
        while let Some(batch) = reader.next().transpose()? {
            rows.extend(decode_batch(&batch)?);
        }
        Ok(rows)
    }
}

fn decode_batch(batch: &RecordBatch) -> anyhow::Result<Vec<StrategyRow>> {
    let schema = batch.schema();
    let col = |name: &str| -> anyhow::Result<usize> {
        schema
            .index_of(name)
            .map_err(|e| anyhow::anyhow!("missing column {name}: {e}"))
    };

    let strategy_ids = batch
        .column(col("strategy_id")?)
        .as_primitive::<Int64Type>();
    let indicator_ids = batch
        .column(col("indicator_id")?)
        .as_primitive::<Int32Type>();
    let rois = batch
        .column(col("annual_roi")?)
        .as_primitive::<Decimal128Type>();
    let draw_downs = batch
        .column(col("max_draw_down")?)
        .as_primitive::<Decimal128Type>();
    let op_counts = batch
        .column(col("annual_operation_count")?)
        .as_primitive::<Decimal128Type>();
    let long_ids = batch
        .column(col("long_operation_point_ids")?)
        .as_list::<i32>();
    let short_ids = batch
        .column(col("short_operation_point_ids")?)
        .as_list::<i32>();
    let mms_ids = batch
        .column(col("money_management_strategy_id")?)
        .as_primitive::<Int32Type>();

    let n = batch.num_rows();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(StrategyRow {
            strategy_id: strategy_ids.value(i),
            indicator_id: indicator_ids.value(i),
            annual_roi: cast::i128_to_f64(rois.value(i)) / 100.0,
            max_draw_down: cast::i128_to_f64(draw_downs.value(i)) / 100.0,
            annual_operation_count: cast::i128_to_f64(op_counts.value(i)) / 100.0,
            long_operation_point_ids: decode_list_value(long_ids, i),
            short_operation_point_ids: decode_list_value(short_ids, i),
            money_management_strategy_id: mms_ids.value(i),
        });
    }
    Ok(out)
}

fn decode_list_value(list: &ListArray, i: usize) -> Vec<i32> {
    let values = list.value(i);
    let prim: &PrimitiveArray<Int32Type> = values.as_primitive();
    prim.values().iter().copied().collect()
}

fn build_record_batch(rows: &[StrategyRow]) -> anyhow::Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("strategy_id", DataType::Int64, false),
        Field::new("indicator_id", DataType::Int32, false),
        Field::new("annual_roi", DataType::Decimal128(10, 2), false),
        Field::new("max_draw_down", DataType::Decimal128(10, 2), false),
        Field::new("annual_operation_count", DataType::Decimal128(10, 2), false),
        Field::new(
            "long_operation_point_ids",
            DataType::List(Arc::new(Field::new("item", DataType::Int32, true))),
            false,
        ),
        Field::new(
            "short_operation_point_ids",
            DataType::List(Arc::new(Field::new("item", DataType::Int32, true))),
            false,
        ),
        Field::new("money_management_strategy_id", DataType::Int32, false),
    ]));

    let strategy_ids: Int64Array = rows.iter().map(|r| r.strategy_id).collect();
    let indicator_ids: Int32Array = rows.iter().map(|r| r.indicator_id).collect();
    let rois = Decimal128Array::from_iter_values(
        rows.iter()
            .map(|r| cast::round_f64_to_i128(r.annual_roi * 100.0)),
    )
    .with_precision_and_scale(10, 2)?;
    let draw_downs = Decimal128Array::from_iter_values(
        rows.iter()
            .map(|r| cast::round_f64_to_i128(r.max_draw_down * 100.0)),
    )
    .with_precision_and_scale(10, 2)?;
    let op_counts = Decimal128Array::from_iter_values(
        rows.iter()
            .map(|r| cast::round_f64_to_i128(r.annual_operation_count * 100.0)),
    )
    .with_precision_and_scale(10, 2)?;
    let mms_ids: Int32Array = rows
        .iter()
        .map(|r| r.money_management_strategy_id)
        .collect();

    let long_ids = build_list_column(rows, |r| &r.long_operation_point_ids);
    let short_ids = build_list_column(rows, |r| &r.short_operation_point_ids);

    Ok(RecordBatch::try_new(
        schema,
        vec![
            Arc::new(strategy_ids),
            Arc::new(indicator_ids),
            Arc::new(rois),
            Arc::new(draw_downs),
            Arc::new(op_counts),
            Arc::new(long_ids),
            Arc::new(short_ids),
            Arc::new(mms_ids),
        ],
    )?)
}

fn build_list_column(rows: &[StrategyRow], extract: fn(&StrategyRow) -> &[i32]) -> ListArray {
    // ListBuilder's default inner field is ("item", Int32, nullable), which
    // matches the schema in build_record_batch.
    let mut builder = ListBuilder::with_capacity(Int32Builder::new(), rows.len());
    for r in rows {
        builder.values().append_slice(extract(r));
        builder.append(true);
    }
    builder.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::strategy::Strategy;
    use tempfile::TempDir;

    fn strategy(indicator_id: i32) -> Strategy {
        Strategy {
            instrument: "EURUSD".to_string(),
            indicator_id,
            annual_roi: 1.0,
            max_draw_down: 2.0,
            annual_operation_count: 3.0,
            long_operation_point_ids: vec![],
            short_operation_point_ids: vec![],
            money_management_strategy_id: 390,
        }
    }

    fn sample_strategies() -> Vec<Strategy> {
        vec![
            Strategy {
                instrument: "EURUSD".to_string(),
                indicator_id: 2001,
                annual_roi: 12.34,
                max_draw_down: 25.67,
                annual_operation_count: 15.2,
                long_operation_point_ids: vec![1, 2, 3],
                short_operation_point_ids: vec![4, 5],
                money_management_strategy_id: 390,
            },
            Strategy {
                instrument: "EURUSD".to_string(),
                indicator_id: 2002,
                annual_roi: -5.10,
                max_draw_down: 35.0,
                annual_operation_count: 12.8,
                long_operation_point_ids: vec![6],
                short_operation_point_ids: vec![7, 8],
                money_management_strategy_id: 390,
            },
        ]
    }

    /// Rows with contiguous strategy ids starting at `first_id`.
    fn rows(strategies: &[Strategy], first_id: i64) -> Vec<StrategyRow> {
        strategies
            .iter()
            .zip(first_id..)
            .map(|(strategy, id)| StrategyRow::from_strategy(strategy, id))
            .collect()
    }

    fn read_back(
        repo: &ParquetStrategyRepository,
        instrument: &str,
        mms_id: i32,
    ) -> Vec<StrategyRow> {
        let path = PathBuf::from(format!(
            "{}/instrument={}/money_management_strategy_id={}/strategies.parquet",
            repo.base_dir.display(),
            instrument,
            mms_id
        ));
        ParquetStrategyRepository::read_rows(&path).expect("read back")
    }

    fn read_back_ids(repo: &ParquetStrategyRepository, instrument: &str, mms_id: i32) -> Vec<i64> {
        read_back(repo, instrument, mms_id)
            .into_iter()
            .map(|r| r.strategy_id)
            .collect()
    }

    fn read_back_indicator_ids(
        repo: &ParquetStrategyRepository,
        instrument: &str,
        mms_id: i32,
    ) -> Vec<i32> {
        read_back(repo, instrument, mms_id)
            .into_iter()
            .map(|r| r.indicator_id)
            .collect()
    }

    #[test]
    fn write_rows_creates_partition_file() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());

        repo.write_rows("EURUSD", 390, rows(&sample_strategies(), 1))
            .unwrap();

        let parquet_path = tmp
            .path()
            .join("instrument=EURUSD/money_management_strategy_id=390/strategies.parquet");
        assert!(parquet_path.exists());
    }

    #[test]
    fn write_rows_creates_one_partition_per_cell() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
        let strategies = sample_strategies();

        repo.write_rows("EURUSD", 390, rows(&strategies, 1))
            .unwrap();
        repo.write_rows("EURUSD", 391, rows(&strategies, 3))
            .unwrap();
        repo.write_rows("USDJPY", 400, rows(&strategies, 5))
            .unwrap();

        assert!(tmp
            .path()
            .join("instrument=EURUSD/money_management_strategy_id=390/strategies.parquet")
            .exists());
        assert!(tmp
            .path()
            .join("instrument=EURUSD/money_management_strategy_id=391/strategies.parquet")
            .exists());
        assert!(tmp
            .path()
            .join("instrument=USDJPY/money_management_strategy_id=400/strategies.parquet")
            .exists());
    }

    #[test]
    fn write_rows_round_trips_every_field() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());

        repo.write_rows("EURUSD", 390, rows(&sample_strategies(), 1))
            .unwrap();

        let back = read_back(&repo, "EURUSD", 390);
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].strategy_id, 1);
        assert_eq!(back[0].indicator_id, 2001);
        assert_eq!(back[0].annual_roi, 12.34);
        assert_eq!(back[0].max_draw_down, 25.67);
        assert_eq!(back[0].annual_operation_count, 15.2);
        assert_eq!(back[0].long_operation_point_ids, vec![1, 2, 3]);
        assert_eq!(back[0].short_operation_point_ids, vec![4, 5]);
        assert_eq!(back[0].money_management_strategy_id, 390);
        assert_eq!(back[1].annual_roi, -5.10);
    }

    #[test]
    fn write_rows_preserves_existing_rows() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());

        repo.write_rows("EURUSD", 390, rows(&sample_strategies(), 1))
            .unwrap();
        repo.write_rows("EURUSD", 390, rows(&[strategy(9999)], 3))
            .unwrap();

        assert_eq!(read_back_ids(&repo, "EURUSD", 390), vec![1, 2, 3]);
        assert_eq!(
            read_back_indicator_ids(&repo, "EURUSD", 390),
            vec![2001, 2002, 9999]
        );
    }

    #[test]
    fn write_rows_deduplicates_same_batch() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
        let strategies = sample_strategies();

        repo.write_rows("EURUSD", 390, rows(&strategies, 1))
            .unwrap();
        repo.write_rows("EURUSD", 390, rows(&strategies, 3))
            .unwrap();

        assert_eq!(read_back_ids(&repo, "EURUSD", 390), vec![1, 2]);
        assert_eq!(
            read_back_indicator_ids(&repo, "EURUSD", 390),
            vec![2001, 2002]
        );
    }

    #[test]
    fn write_rows_partial_overlap_ignores_duplicate() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
        let overlapping = vec![
            Strategy {
                annual_roi: 99.0,
                long_operation_point_ids: vec![100],
                ..strategy(2002)
            },
            strategy(3000),
        ];

        repo.write_rows("EURUSD", 390, rows(&sample_strategies(), 1))
            .unwrap();
        repo.write_rows("EURUSD", 390, rows(&overlapping, 3))
            .unwrap();

        // The duplicate 2002 keeps its original row (id 2); only 3000 is added.
        assert_eq!(read_back_ids(&repo, "EURUSD", 390), vec![1, 2, 4]);
        assert_eq!(
            read_back_indicator_ids(&repo, "EURUSD", 390),
            vec![2001, 2002, 3000]
        );
        assert_eq!(read_back(&repo, "EURUSD", 390)[1].annual_roi, -5.10);
    }

    #[test]
    fn write_rows_does_not_touch_other_partitions() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
        let strategies = sample_strategies();

        repo.write_rows("EURUSD", 390, rows(&strategies, 1))
            .unwrap();
        repo.write_rows("USDJPY", 400, rows(&strategies, 3))
            .unwrap();
        repo.write_rows("EURUSD", 390, rows(&strategies, 5))
            .unwrap();

        assert_eq!(read_back_ids(&repo, "USDJPY", 400), vec![3, 4]);
    }

    #[test]
    fn max_strategy_id_is_zero_before_any_write() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().join("missing"));

        assert_eq!(repo.max_strategy_id().unwrap(), 0);
    }

    #[test]
    fn max_strategy_id_spans_all_partitions() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
        let strategies = sample_strategies();

        repo.write_rows("EURUSD", 390, rows(&strategies, 1))
            .unwrap();
        repo.write_rows("USDJPY", 400, rows(&strategies, 7))
            .unwrap();
        repo.write_rows("EURUSD", 391, rows(&strategies, 3))
            .unwrap();

        assert_eq!(repo.max_strategy_id().unwrap(), 8);
    }
}
