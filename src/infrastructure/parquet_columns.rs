//! Checked column access for Parquet files the engine reads back.
//!
//! Arrow's `as_primitive`/`as_list` panic on a type mismatch, and the release
//! profile aborts on panic, so one drifted file would kill a whole batch run.
//! Every read of an on-disk column goes through here and fails as an error.

use anyhow::Context;
use arrow::array::{ArrayRef, AsArray, ListArray, PrimitiveArray, StringArray};
use arrow::datatypes::ArrowPrimitiveType;
use arrow::record_batch::RecordBatch;

pub fn column<'a>(batch: &'a RecordBatch, name: &str) -> anyhow::Result<&'a ArrayRef> {
    batch
        .column_by_name(name)
        .with_context(|| format!("missing column {name}"))
}

pub fn string_column<'a>(batch: &'a RecordBatch, name: &str) -> anyhow::Result<&'a StringArray> {
    column(batch, name)?
        .as_string_opt::<i32>()
        .with_context(|| format!("column {name} is not Utf8"))
}

pub fn primitive_column<'a, T: ArrowPrimitiveType>(
    batch: &'a RecordBatch,
    name: &str,
) -> anyhow::Result<&'a PrimitiveArray<T>> {
    column(batch, name)?
        .as_primitive_opt::<T>()
        .with_context(|| format!("column {name} is not {}", T::DATA_TYPE))
}

pub fn list_column<'a>(batch: &'a RecordBatch, name: &str) -> anyhow::Result<&'a ListArray> {
    column(batch, name)?
        .as_list_opt::<i32>()
        .with_context(|| format!("column {name} is not a List"))
}

pub fn list_items<T: ArrowPrimitiveType>(
    list: &ListArray,
    row: usize,
    name: &str,
) -> anyhow::Result<Vec<T::Native>> {
    let items = list.value(row);
    let items = items
        .as_primitive_opt::<T>()
        .with_context(|| format!("column {name} items are not {}", T::DATA_TYPE))?;
    Ok(items.values().to_vec())
}
