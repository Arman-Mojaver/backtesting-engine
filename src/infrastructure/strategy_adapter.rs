use std::fs;
use std::fs::File;
use std::sync::Arc;

use arrow::array::{Float64Array, Int32Array, Int32Builder, Int64Array, ListBuilder};
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
    /// Create a new repository rooted at `base_dir` (from `parquet_base_dir()`).
    pub fn new(base_dir: String) -> Self {
        ParquetStrategyRepository { base_dir }
    }

    /// Write the given strategies to a parquet file at the correct partition path.
    ///
    /// The write is safe: data is first written to a `.tmp` file, then atomically
    /// renamed over the final path. If the process crashes mid-write, the original
    /// file is untouched.
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

        let final_path = format!("{}/strategies.parquet", partition_dir);
        let tmp_path = format!("{}.tmp", final_path);

        let file = File::create(&tmp_path)?;
        let batch = build_record_batch(strategies)?;
        let schema = batch.schema();

        let props = WriterProperties::builder()
            .set_compression(Compression::ZSTD(Default::default()))
            .build();

        let mut writer = ArrowWriter::try_new(file, schema, Some(props))?;
        writer.write(&batch)?;
        writer.close()?;

        fs::rename(&tmp_path, &final_path)?;
        Ok(())
    }
}

/// Build a single `RecordBatch` from a slice of `Strategy` values.
fn build_record_batch(strategies: &[Strategy]) -> anyhow::Result<RecordBatch> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("strategy_id", DataType::Int64, false),
        Field::new("indicator_id", DataType::Int32, false),
        Field::new("annual_roi", DataType::Float64, false),
        Field::new("max_draw_down", DataType::Float64, false),
        Field::new("annual_operation_count", DataType::Float64, false),
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
    ]));

    let strategy_ids: Int64Array = strategies.iter().map(|s| s.strategy_id).collect();
    let indicator_ids: Int32Array = strategies.iter().map(|s| s.indicator_id).collect();
    let rois: Float64Array = strategies.iter().map(|s| s.annual_roi).collect();
    let draw_downs: Float64Array = strategies.iter().map(|s| s.max_draw_down).collect();
    let op_counts: Float64Array = strategies
        .iter()
        .map(|s| s.annual_operation_count)
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
        ],
    )?)
}

/// Build an Arrow `List<Int32>` column from a slice of strategies, extracting
/// the ID list via the given closure.
fn build_list_column(
    strategies: &[Strategy],
    extract: fn(&Strategy) -> &[i32],
) -> arrow::array::ListArray {
    let mut builder = ListBuilder::new(Int32Builder::new());
    for s in strategies {
        let ids = extract(s);
        if ids.is_empty() {
            builder.append(false);
        } else {
            builder.append(true);
            for id in ids {
                builder.values().append_value(*id);
            }
        }
    }
    builder.finish()
}
