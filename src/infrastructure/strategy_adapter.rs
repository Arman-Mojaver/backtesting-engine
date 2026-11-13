use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{bail, Context};
use arrow::array::{
    AsArray, Decimal128Array, Int32Array, Int32Builder, Int64Array, ListArray, ListBuilder,
    PrimitiveArray,
};
use arrow::datatypes::{DataType, Decimal128Type, Field, Int32Type, Int64Type, Schema};
use arrow::record_batch::RecordBatch;
use log::debug;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::{ArrowWriter, ProjectionMask};
use parquet::basic::{Compression, ZstdLevel};
use parquet::file::properties::WriterProperties;
use rustc_hash::FxHashSet;

use crate::domain::cast;
use crate::domain::cell::Cell;
use crate::domain::strategy::Strategy;

const METRIC_PRECISION: u8 = 10;
const METRIC_SCALE: i8 = 2;
const METRIC_TYPE: DataType = DataType::Decimal128(METRIC_PRECISION, METRIC_SCALE);

fn metric_scale_factor() -> f64 {
    10_f64.powi(i32::from(METRIC_SCALE))
}

fn metric_from_decimal(raw: i128) -> f64 {
    cast::i128_to_f64(raw) / metric_scale_factor()
}

fn metric_array(values: impl Iterator<Item = f64>) -> anyhow::Result<Decimal128Array> {
    let raw = values.map(|v| cast::round_f64_to_i128(v * metric_scale_factor()));
    Ok(Decimal128Array::from_iter_values(raw)
        .with_precision_and_scale(METRIC_PRECISION, METRIC_SCALE)?)
}

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
    pub fn from_strategy(s: Strategy, id: i64) -> Self {
        Self {
            strategy_id: id,
            indicator_id: s.indicator_id,
            annual_roi: s.annual_roi,
            max_draw_down: s.max_draw_down,
            annual_operation_count: s.annual_operation_count,
            long_operation_point_ids: s.long_operation_point_ids,
            short_operation_point_ids: s.short_operation_point_ids,
            money_management_strategy_id: s.money_management_strategy_id,
        }
    }
}

#[derive(Debug)]
pub struct ParquetStrategyRepository {
    base_dir: PathBuf,
}

impl ParquetStrategyRepository {
    pub const fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    pub fn write_rows(&self, cell: &Cell, rows: Vec<StrategyRow>) -> anyhow::Result<()> {
        let path = self.partition_path(cell);
        Self::write_partition(&path, rows)
    }

    fn partition_path(&self, cell: &Cell) -> PathBuf {
        self.base_dir
            .join(format!("instrument={}", cell.instrument))
            .join(format!("money_management_strategy_id={}", cell.mms_id))
            .join("strategies.parquet")
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

        let Some(partition_dir) = path.parent() else {
            bail!("partition {} has no parent directory", path.display());
        };
        fs::create_dir_all(partition_dir).with_context(|| {
            format!(
                "failed to create partition directory {}",
                partition_dir.display()
            )
        })?;

        // Intentional: the partition is rewritten in place, not through a temp
        // file and rename. A write that fails midway is an accepted risk: the
        // command is re-run, with or without fixes, and strategies are derived
        // from PostgreSQL, so nothing is lost.
        let file = File::create(path)
            .with_context(|| format!("failed to create partition {}", path.display()))?;
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
            let entries = fs::read_dir(&dir)
                .with_context(|| format!("failed to read directory {}", dir.display()))?;
            for entry in entries {
                let entry =
                    entry.with_context(|| format!("failed to read directory {}", dir.display()))?;
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.file_name().is_some_and(|n| n == "strategies.parquet") {
                    partitions_scanned = partitions_scanned.saturating_add(1);
                    max_id = max_id.max(Self::partition_max_strategy_id(&path)?);
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

    fn partition_max_strategy_id(path: &Path) -> anyhow::Result<i64> {
        let file = File::open(path)
            .with_context(|| format!("failed to open partition {}", path.display()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)?;
        let strategy_id_index = builder
            .schema()
            .index_of("strategy_id")
            .with_context(|| format!("partition {} has no strategy_id column", path.display()))?;
        let mask = ProjectionMask::roots(builder.parquet_schema(), [strategy_id_index]);
        let mut reader = builder.with_projection(mask).build()?;

        let mut max_id: i64 = 0;
        while let Some(batch) = reader.next().transpose()? {
            for id in batch.column(0).as_primitive::<Int64Type>().values() {
                max_id = max_id.max(*id);
            }
        }
        Ok(max_id)
    }

    fn read_rows(path: &Path) -> anyhow::Result<Vec<StrategyRow>> {
        let file = File::open(path)
            .with_context(|| format!("failed to open partition {}", path.display()))?;
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
            annual_roi: metric_from_decimal(rois.value(i)),
            max_draw_down: metric_from_decimal(draw_downs.value(i)),
            annual_operation_count: metric_from_decimal(op_counts.value(i)),
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
        Field::new("annual_roi", METRIC_TYPE, false),
        Field::new("max_draw_down", METRIC_TYPE, false),
        Field::new("annual_operation_count", METRIC_TYPE, false),
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
    let rois = metric_array(rows.iter().map(|r| r.annual_roi))?;
    let draw_downs = metric_array(rows.iter().map(|r| r.max_draw_down))?;
    let op_counts = metric_array(rows.iter().map(|r| r.annual_operation_count))?;
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
            .cloned()
            .zip(first_id..)
            .map(|(strategy, id)| StrategyRow::from_strategy(strategy, id))
            .collect()
    }

    fn read_back(repo: &ParquetStrategyRepository, cell: &Cell) -> Vec<StrategyRow> {
        ParquetStrategyRepository::read_rows(&repo.partition_path(cell)).expect("read back")
    }

    fn read_back_ids(repo: &ParquetStrategyRepository, cell: &Cell) -> Vec<i64> {
        read_back(repo, cell)
            .into_iter()
            .map(|r| r.strategy_id)
            .collect()
    }

    fn read_back_indicator_ids(repo: &ParquetStrategyRepository, cell: &Cell) -> Vec<i32> {
        read_back(repo, cell)
            .into_iter()
            .map(|r| r.indicator_id)
            .collect()
    }

    #[test]
    fn write_rows_creates_partition_file() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());

        repo.write_rows(&Cell::new("EURUSD", 390), rows(&sample_strategies(), 1))
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

        repo.write_rows(&Cell::new("EURUSD", 390), rows(&strategies, 1))
            .unwrap();
        repo.write_rows(&Cell::new("EURUSD", 391), rows(&strategies, 3))
            .unwrap();
        repo.write_rows(&Cell::new("USDJPY", 400), rows(&strategies, 5))
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

        repo.write_rows(&Cell::new("EURUSD", 390), rows(&sample_strategies(), 1))
            .unwrap();

        let back = read_back(&repo, &Cell::new("EURUSD", 390));
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
    fn write_rows_stores_metrics_as_rounded_two_decimal_integers() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
        let mut strategies = sample_strategies();
        strategies[1].annual_roi = 12.346;
        strategies[1].max_draw_down = 0.004;
        strategies[1].annual_operation_count = -0.006;

        repo.write_rows(&Cell::new("EURUSD", 390), rows(&strategies, 1))
            .unwrap();

        let file = File::open(repo.partition_path(&Cell::new("EURUSD", 390))).unwrap();
        let batch = ParquetRecordBatchReaderBuilder::try_new(file)
            .unwrap()
            .build()
            .unwrap()
            .next()
            .unwrap()
            .unwrap();
        let raw = |name: &str| -> (DataType, Vec<i128>) {
            let column = batch.column_by_name(name).unwrap();
            let values = column.as_primitive::<Decimal128Type>().values().to_vec();
            (column.data_type().clone(), values)
        };
        let decimal_10_2 = DataType::Decimal128(10, 2);
        assert_eq!(raw("annual_roi"), (decimal_10_2.clone(), vec![1234, 1235]));
        assert_eq!(raw("max_draw_down"), (decimal_10_2.clone(), vec![2567, 0]));
        assert_eq!(
            raw("annual_operation_count"),
            (decimal_10_2, vec![1520, -1])
        );
    }

    #[test]
    fn write_rows_preserves_existing_rows() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());

        repo.write_rows(&Cell::new("EURUSD", 390), rows(&sample_strategies(), 1))
            .unwrap();
        repo.write_rows(&Cell::new("EURUSD", 390), rows(&[strategy(9999)], 3))
            .unwrap();

        assert_eq!(
            read_back_ids(&repo, &Cell::new("EURUSD", 390)),
            vec![1, 2, 3]
        );
        assert_eq!(
            read_back_indicator_ids(&repo, &Cell::new("EURUSD", 390)),
            vec![2001, 2002, 9999]
        );
    }

    #[test]
    fn write_rows_deduplicates_same_batch() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
        let strategies = sample_strategies();

        repo.write_rows(&Cell::new("EURUSD", 390), rows(&strategies, 1))
            .unwrap();
        repo.write_rows(&Cell::new("EURUSD", 390), rows(&strategies, 3))
            .unwrap();

        assert_eq!(read_back_ids(&repo, &Cell::new("EURUSD", 390)), vec![1, 2]);
        assert_eq!(
            read_back_indicator_ids(&repo, &Cell::new("EURUSD", 390)),
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

        repo.write_rows(&Cell::new("EURUSD", 390), rows(&sample_strategies(), 1))
            .unwrap();
        repo.write_rows(&Cell::new("EURUSD", 390), rows(&overlapping, 3))
            .unwrap();

        // The duplicate 2002 keeps its original row (id 2); only 3000 is added.
        assert_eq!(
            read_back_ids(&repo, &Cell::new("EURUSD", 390)),
            vec![1, 2, 4]
        );
        assert_eq!(
            read_back_indicator_ids(&repo, &Cell::new("EURUSD", 390)),
            vec![2001, 2002, 3000]
        );
        assert_eq!(
            read_back(&repo, &Cell::new("EURUSD", 390))[1].annual_roi,
            -5.10
        );
    }

    #[test]
    fn write_rows_does_not_touch_other_partitions() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
        let strategies = sample_strategies();

        repo.write_rows(&Cell::new("EURUSD", 390), rows(&strategies, 1))
            .unwrap();
        repo.write_rows(&Cell::new("USDJPY", 400), rows(&strategies, 3))
            .unwrap();
        repo.write_rows(&Cell::new("EURUSD", 390), rows(&strategies, 5))
            .unwrap();

        assert_eq!(read_back_ids(&repo, &Cell::new("USDJPY", 400)), vec![3, 4]);
    }

    #[test]
    fn write_rows_names_the_partition_directory_it_cannot_create() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
        let blocking_file = tmp.path().join("instrument=EURUSD");
        fs::write(&blocking_file, b"").unwrap();

        let err = repo
            .write_rows(&Cell::new("EURUSD", 390), rows(&sample_strategies(), 1))
            .unwrap_err();

        assert!(err
            .to_string()
            .contains(&blocking_file.display().to_string()));
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

        repo.write_rows(&Cell::new("EURUSD", 390), rows(&strategies, 1))
            .unwrap();
        repo.write_rows(&Cell::new("USDJPY", 400), rows(&strategies, 7))
            .unwrap();
        repo.write_rows(&Cell::new("EURUSD", 391), rows(&strategies, 3))
            .unwrap();

        assert_eq!(repo.max_strategy_id().unwrap(), 8);
    }

    #[test]
    fn max_strategy_id_finds_the_column_by_name() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_path_buf());
        let path = repo.partition_path(&Cell::new("EURUSD", 390));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let schema = Arc::new(Schema::new(vec![
            Field::new("indicator_id", DataType::Int64, false),
            Field::new("strategy_id", DataType::Int64, false),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![2001, 2002])),
                Arc::new(Int64Array::from(vec![7, 3])),
            ],
        )
        .unwrap();
        let file = File::create(path).unwrap();
        let mut writer = ArrowWriter::try_new(file, schema, None).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();

        assert_eq!(repo.max_strategy_id().unwrap(), 7);
    }
}
