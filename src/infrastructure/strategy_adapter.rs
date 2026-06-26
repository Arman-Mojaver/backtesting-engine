use std::fs;
use std::fs::File;
use std::sync::Arc;

use arrow::array::{Decimal128Array, Int32Array, Int64Array, ListArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::file::properties::WriterProperties;

use crate::domain::strategy::Strategy;

/// Writes `Vec<Strategy>` to partitioned parquet files.
///
/// Partition layout:
///   {base_dir}/instrument={instrument}/money_management_strategy_id={mms_id}/strategies.parquet
pub struct ParquetStrategyRepository {
    base_dir: String,
}

impl ParquetStrategyRepository {
    pub fn new(base_dir: String) -> Self {
        ParquetStrategyRepository { base_dir }
    }

    /// Write strategies directly to a parquet file at the correct partition path.
    pub fn write(
        &self,
        instrument: &str,
        mms_id: i32,
        strategies: &[Strategy],
    ) -> anyhow::Result<()> {
        let partition_dir = format!(
            "{}/instrument={}/money_management_strategy_id={}",
            self.base_dir, instrument, mms_id
        );
        fs::create_dir_all(&partition_dir)?;

        let path = format!("{}/strategies.parquet", partition_dir);

        let file = File::create(&path)?;
        let batch = build_record_batch(strategies)?;
        let schema = batch.schema();

        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(Default::default()))
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        Ok(())
    }
}

fn build_record_batch(strategies: &[Strategy]) -> anyhow::Result<RecordBatch> {
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

    let strategy_ids: Int64Array = strategies.iter().map(|s| s.strategy_id).collect();
    let indicator_ids: Int32Array = strategies.iter().map(|s| s.indicator_id).collect();
    let rois = Decimal128Array::from_iter_values(
        strategies
            .iter()
            .map(|s| (s.annual_roi * 100.0).round() as i128),
    )
    .with_precision_and_scale(10, 2)?;
    let draw_downs = Decimal128Array::from_iter_values(
        strategies
            .iter()
            .map(|s| (s.max_draw_down * 100.0).round() as i128),
    )
    .with_precision_and_scale(10, 2)?;
    let op_counts = Decimal128Array::from_iter_values(
        strategies
            .iter()
            .map(|s| (s.annual_operation_count * 100.0).round() as i128),
    )
    .with_precision_and_scale(10, 2)?;
    let mms_ids: Int32Array = strategies
        .iter()
        .map(|s| s.money_management_strategy_id)
        .collect();

    let long_ids = build_list_column(strategies, |s| &s.long_operation_point_ids);
    let short_ids = build_list_column(strategies, |s| &s.short_operation_point_ids);

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

fn build_list_column(strategies: &[Strategy], extract: fn(&Strategy) -> &[i32]) -> ListArray {
    use arrow::array::Int32Builder;
    let mut values_builder = Int32Builder::new();
    let mut offsets: Vec<i32> = vec![0];
    for s in strategies {
        let ids = extract(s);
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
                strategy_id: 10001,
                indicator_id: 2001,
                annual_roi: 12.34,
                max_draw_down: 25.67,
                annual_operation_count: 15.2,
                long_operation_point_ids: vec![1, 2, 3],
                short_operation_point_ids: vec![4, 5],
                money_management_strategy_id: 390,
            },
            Strategy {
                strategy_id: 10002,
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
}
