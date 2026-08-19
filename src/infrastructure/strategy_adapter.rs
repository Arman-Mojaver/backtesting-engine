use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use arrow::array::{
    AsArray, Decimal128Array, Int32Array, Int32Builder, Int64Array, ListArray, PrimitiveArray,
};
use arrow::datatypes::{DataType, Decimal128Type, Field, Int32Type, Int64Type, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;
use rustc_hash::FxHashSet;

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
        StrategyRow {
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
    base_dir: String,
}

impl ParquetStrategyRepository {
    pub fn new(base_dir: String) -> Self {
        ParquetStrategyRepository { base_dir }
    }

    pub fn write(
        &self,
        instrument: &str,
        mms_id: i32,
        strategies: &[Strategy],
    ) -> anyhow::Result<()> {
        let mut next_id = self.max_strategy_id()? + 1;
        let mut rows = Vec::with_capacity(strategies.len());
        for strategy in strategies {
            rows.push(StrategyRow::from_strategy(strategy, next_id));
            next_id += 1;
        }
        self.write_rows(instrument, mms_id, rows)
    }

    pub fn write_rows(
        &self,
        instrument: &str,
        mms_id: i32,
        rows: Vec<StrategyRow>,
    ) -> anyhow::Result<()> {
        let path = self.partition_path(instrument, mms_id)?;
        self.write_partition(&path, rows)
    }

    /// Returns the parquet path for a single `(instrument, mms_id)` partition,
    /// creating the partition directory if needed.
    fn partition_path(&self, instrument: &str, mms_id: i32) -> anyhow::Result<PathBuf> {
        let partition_dir = format!(
            "{}/instrument={}/money_management_strategy_id={}",
            self.base_dir, instrument, mms_id
        );
        fs::create_dir_all(&partition_dir)?;
        Ok(PathBuf::from(format!(
            "{}/strategies.parquet",
            partition_dir
        )))
    }

    fn write_partition(&self, path: &Path, new_rows: Vec<StrategyRow>) -> anyhow::Result<()> {
        let existing_rows: Vec<StrategyRow> = if path.exists() {
            self.read_rows(path)?
        } else {
            Vec::new()
        };

        let existing_ids: FxHashSet<i32> = existing_rows.iter().map(|r| r.indicator_id).collect();
        let fresh_rows: Vec<StrategyRow> = new_rows
            .into_iter()
            .filter(|row| !existing_ids.contains(&row.indicator_id))
            .collect();

        if fresh_rows.is_empty() {
            return Ok(());
        }

        let mut combined = existing_rows;
        combined.extend(fresh_rows);

        let batch = build_record_batch(&combined)?;
        let schema = batch.schema();

        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(Default::default()))
            .build();

        let file = File::create(path)?;
        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        Ok(())
    }

    pub fn max_strategy_id(&self) -> anyhow::Result<i64> {
        let root = PathBuf::from(&self.base_dir);
        if !root.exists() {
            return Ok(0);
        }
        let mut max_id: i64 = 0;
        let mut dirs: Vec<PathBuf> = vec![root];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(&dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.file_name().is_some_and(|n| n == "strategies.parquet") {
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
        Ok(max_id)
    }

    fn read_rows(&self, path: &Path) -> anyhow::Result<Vec<StrategyRow>> {
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
            annual_roi: rois.value(i) as f64 / 100.0,
            max_draw_down: draw_downs.value(i) as f64 / 100.0,
            annual_operation_count: op_counts.value(i) as f64 / 100.0,
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
        rows.iter().map(|r| (r.annual_roi * 100.0).round() as i128),
    )
    .with_precision_and_scale(10, 2)?;
    let draw_downs = Decimal128Array::from_iter_values(
        rows.iter()
            .map(|r| (r.max_draw_down * 100.0).round() as i128),
    )
    .with_precision_and_scale(10, 2)?;
    let op_counts = Decimal128Array::from_iter_values(
        rows.iter()
            .map(|r| (r.annual_operation_count * 100.0).round() as i128),
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
    let mut values_builder = Int32Builder::new();
    let mut offsets: Vec<i32> = vec![0];
    for r in rows {
        let ids = extract(r);
        for id in ids {
            values_builder.append_value(*id);
        }
        offsets.push(offsets.last().unwrap() + ids.len() as i32);
    }
    let values = values_builder.finish();
    let offset_buf = arrow::buffer::OffsetBuffer::new(arrow::buffer::ScalarBuffer::from(offsets));
    ListArray::new(
        arrow::datatypes::Field::new("item", arrow::datatypes::DataType::Int32, true).into(),
        offset_buf,
        Arc::new(values),
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::strategy::Strategy;
    use tempfile::TempDir;

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

    fn read_back_ids(repo: &ParquetStrategyRepository, instrument: &str, mms_id: i32) -> Vec<i64> {
        let path = PathBuf::from(format!(
            "{}/instrument={}/money_management_strategy_id={}/strategies.parquet",
            repo.base_dir, instrument, mms_id
        ));
        let rows = repo.read_rows(&path).expect("read back");
        rows.into_iter().map(|r| r.strategy_id).collect()
    }

    fn read_back_indicator_ids(
        repo: &ParquetStrategyRepository,
        instrument: &str,
        mms_id: i32,
    ) -> Vec<i32> {
        let path = PathBuf::from(format!(
            "{}/instrument={}/money_management_strategy_id={}/strategies.parquet",
            repo.base_dir, instrument, mms_id
        ));
        let rows = repo.read_rows(&path).expect("read back");
        rows.into_iter().map(|r| r.indicator_id).collect()
    }

    #[test]
    fn write_does_not_error() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let strategies = sample_strategies();

        repo.write("EURUSD", 390, &strategies).unwrap();

        let parquet_path = tmp
            .path()
            .join("instrument=EURUSD/money_management_strategy_id=390/strategies.parquet");
        assert!(parquet_path.exists());
    }

    #[test]
    fn write_multiple_partitions() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let strategies = sample_strategies();

        repo.write("EURUSD", 390, &strategies).unwrap();
        repo.write("EURUSD", 391, &strategies).unwrap();
        repo.write("USDJPY", 400, &strategies).unwrap();

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
    fn first_write_starts_ids_at_one() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let strategies = sample_strategies();

        repo.write("EURUSD", 390, &strategies).unwrap();

        let ids = read_back_ids(&repo, "EURUSD", 390);
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn second_write_continues_from_global_max() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let strategies = sample_strategies();

        repo.write("EURUSD", 390, &strategies).unwrap();
        repo.write("USDJPY", 400, &strategies).unwrap();

        let ids_a = read_back_ids(&repo, "EURUSD", 390);
        let ids_b = read_back_ids(&repo, "USDJPY", 400);
        assert_eq!(ids_a, vec![1, 2]);
        assert_eq!(ids_b, vec![3, 4]);
    }

    #[test]
    fn global_max_across_multiple_partitions() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let strategies = sample_strategies();

        repo.write("EURUSD", 390, &strategies).unwrap();
        repo.write("EURUSD", 391, &strategies).unwrap();
        repo.write("USDJPY", 400, &strategies).unwrap();

        let ids = read_back_ids(&repo, "USDJPY", 400);
        assert_eq!(ids, vec![5, 6]);
    }

    #[test]
    fn sequential_ids_are_contiguous_within_a_batch() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let strategies: Vec<Strategy> = (0..5)
            .map(|i| Strategy {
                instrument: "EURUSD".to_string(),
                indicator_id: i as i32,
                annual_roi: 10.0,
                max_draw_down: 20.0,
                annual_operation_count: 12.0,
                long_operation_point_ids: vec![i as i32],
                short_operation_point_ids: vec![],
                money_management_strategy_id: 1,
            })
            .collect();

        repo.write("EURUSD", 1, &strategies).unwrap();

        let ids = read_back_ids(&repo, "EURUSD", 1);
        assert_eq!(ids, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn append_preserves_existing_rows() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let first = sample_strategies();
        let second: Vec<Strategy> = vec![Strategy {
            instrument: "EURUSD".to_string(),
            indicator_id: 9999,
            annual_roi: 1.0,
            max_draw_down: 2.0,
            annual_operation_count: 3.0,
            long_operation_point_ids: vec![],
            short_operation_point_ids: vec![],
            money_management_strategy_id: 390,
        }];

        repo.write("EURUSD", 390, &first).unwrap();
        repo.write("EURUSD", 390, &second).unwrap();

        let ids = read_back_ids(&repo, "EURUSD", 390);
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn rewriting_same_partition_preserves_original_ids() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let strategies = sample_strategies();

        repo.write("EURUSD", 390, &strategies).unwrap();
        repo.write("USDJPY", 400, &strategies).unwrap();
        repo.write("EURUSD", 390, &strategies).unwrap();

        // Re-writing an already-written partition is a no-op: original IDs are preserved.
        let ids = read_back_ids(&repo, "EURUSD", 390);
        assert_eq!(ids, vec![1, 2]);
    }

    // ---------- Dedup-on-write tests ----------

    #[test]
    fn write_deduplicates_same_batch() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let strategies = sample_strategies();

        repo.write("EURUSD", 390, &strategies).unwrap();
        repo.write("EURUSD", 390, &strategies).unwrap();

        let indicator_ids = read_back_indicator_ids(&repo, "EURUSD", 390);
        assert_eq!(indicator_ids.len(), 2);
        assert!(indicator_ids.contains(&2001));
        assert!(indicator_ids.contains(&2002));
    }

    #[test]
    fn write_preserves_different_indicators() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let first = sample_strategies();
        let second: Vec<Strategy> = vec![Strategy {
            instrument: "EURUSD".to_string(),
            indicator_id: 9999,
            annual_roi: 1.0,
            max_draw_down: 2.0,
            annual_operation_count: 3.0,
            long_operation_point_ids: vec![],
            short_operation_point_ids: vec![],
            money_management_strategy_id: 390,
        }];

        repo.write("EURUSD", 390, &first).unwrap();
        repo.write("EURUSD", 390, &second).unwrap();

        let indicator_ids = read_back_indicator_ids(&repo, "EURUSD", 390);
        assert_eq!(indicator_ids.len(), 3);
        assert!(indicator_ids.contains(&2001));
        assert!(indicator_ids.contains(&2002));
        assert!(indicator_ids.contains(&9999));
    }

    #[test]
    fn write_partial_overlap_ignores_duplicate() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let first = sample_strategies();
        let second: Vec<Strategy> = vec![
            Strategy {
                instrument: "EURUSD".to_string(),
                indicator_id: 2002,
                annual_roi: 99.0,
                max_draw_down: 1.0,
                annual_operation_count: 5.0,
                long_operation_point_ids: vec![100],
                short_operation_point_ids: vec![],
                money_management_strategy_id: 390,
            },
            Strategy {
                instrument: "EURUSD".to_string(),
                indicator_id: 3000,
                annual_roi: 1.0,
                max_draw_down: 2.0,
                annual_operation_count: 3.0,
                long_operation_point_ids: vec![],
                short_operation_point_ids: vec![],
                money_management_strategy_id: 390,
            },
        ];

        repo.write("EURUSD", 390, &first).unwrap();
        repo.write("EURUSD", 390, &second).unwrap();

        // The duplicate 2002 is ignored (original id 2 preserved); only 3000 is added
        // (it receives id 4, since id 3 was pre-assigned to the ignored duplicate).
        assert_eq!(read_back_ids(&repo, "EURUSD", 390), vec![1, 2, 4]);
        assert_eq!(
            read_back_indicator_ids(&repo, "EURUSD", 390),
            vec![2001, 2002, 3000]
        );
    }

    #[test]
    fn dedup_does_not_affect_ids_from_other_partitions() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());
        let strategies = sample_strategies();

        repo.write("EURUSD", 390, &strategies).unwrap();
        repo.write("USDJPY", 400, &strategies).unwrap();
        repo.write("EURUSD", 390, &strategies).unwrap();

        let usdjpy_ids = read_back_ids(&repo, "USDJPY", 400);
        assert_eq!(usdjpy_ids, vec![3, 4]);
    }

    #[test]
    fn write_rows_ignores_duplicate_incoming_rows() {
        let tmp = TempDir::new().unwrap();
        let repo = ParquetStrategyRepository::new(tmp.path().to_string_lossy().to_string());

        fn s(indicator_id: i32) -> Strategy {
            Strategy {
                instrument: "EURUSD".to_string(),
                indicator_id,
                annual_roi: 1.0,
                max_draw_down: 1.0,
                annual_operation_count: 1.0,
                long_operation_point_ids: vec![],
                short_operation_point_ids: vec![],
                money_management_strategy_id: 390,
            }
        }

        // First write assigns id 1 to indicator 2001.
        repo.write("EURUSD", 390, &[s(2001)]).unwrap();

        // Re-write the same indicator with a would-be new id 100: it must be ignored.
        let duplicate = StrategyRow::from_strategy(&s(2001), 100);
        repo.write_rows("EURUSD", 390, vec![duplicate]).unwrap();

        // Original row (id 1) is preserved; incoming id 100 discarded.
        assert_eq!(read_back_ids(&repo, "EURUSD", 390), vec![1]);
    }
}
