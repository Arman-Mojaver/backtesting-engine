//! Deep Dive: parquet_columns — checked column access for on-disk Parquet
//! Source: Code input — src/infrastructure/parquet_columns.rs
//!
//! Arrow's `as_primitive`/`as_list` downcasts panic on a type mismatch, and the
//! release profile sets `panic = "abort"`. One drifted Parquet file would kill a
//! whole batch run. These helpers use the `_opt` downcasts instead and turn
//! every mismatch into an `anyhow::Error` naming the offending column.
//! Callers: processed_manifest.rs (manifest rows) and strategy_adapter.rs
//! (strategy rows, including the List<Int32> operation-point id columns).

use std::env;
use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, AsArray, Decimal128Array, Int32Array, Int32Builder, Int64Array,
    Int64Builder, ListBuilder, StringArray,
};
use arrow::datatypes::{ArrowPrimitiveType, Decimal128Type, Int32Type, Int64Type};
use arrow::record_batch::RecordBatch;
use backtesting_engine::infrastructure::parquet_columns::{
    column, list_column, list_items, primitive_column, string_column,
};

// ---- Seed data: a manifest-like batch, plus a "drifted" twin ----
// The writer stores mms_id as Int32. The drifted batch is what an older or
// hand-edited file would look like: mms_id widened to Int64, instrument stored
// as an id instead of a string.
fn manifest_batch() -> RecordBatch {
    RecordBatch::try_from_iter([
        ("instrument", Arc::new(StringArray::from(vec!["EURUSD", "USDJPY"])) as ArrayRef),
        ("mms_id", Arc::new(Int32Array::from(vec![390, 400])) as ArrayRef),
        ("processed_at", Arc::new(Int64Array::from(vec![1_731_974_400, 1_731_978_000])) as ArrayRef),
    ])
    .expect("seed batch is well-formed")
}

fn drifted_batch() -> RecordBatch {
    RecordBatch::try_from_iter([
        ("instrument", Arc::new(Int32Array::from(vec![7, 8])) as ArrayRef),
        ("mms_id", Arc::new(Int64Array::from(vec![390_i64, 400])) as ArrayRef),
    ])
    .expect("seed batch is well-formed")
}

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_column_lookup(),
        "2" => section_2_string_column(),
        "3" => section_3_primitive_width_drift(),
        "4" => section_4_decimal_parameters(),
        "5" => section_5_list_items(),
        "6" => section_6_list_items_caveats(),
        "7" => section_7_why_not_as_primitive(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-parquet-columns <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Column Lookup: column() and the missing-column error");
    println!("  2    - String Column: Utf8 happy path vs an Int32 impostor");
    println!("  3    - Primitive Width Drift: Int32 expected, Int64 on disk");
    println!("  4    - Decimal Parameters: what primitive_column does NOT check");
    println!("  5    - List Items: list_column + list_items per row");
    println!("  6    - List Items Caveats: mistyped items, nulls, row bounds");
    println!("  7    - Why Not as_primitive: the panic these helpers avoid");
}

// ============================================================
// SECTION 1: Column Lookup - column() and the missing-column error
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_column_lookup() {
    println!("============================================================");
    println!("SECTION 1: Column Lookup - column() and the missing-column error");
    println!("============================================================");
    let batch = manifest_batch();

    // column() is the single choke point: every typed helper calls it first,
    // so "missing" is reported the same way regardless of the expected type.
    let mms = column(&batch, "mms_id").expect("present");
    println!("column(\"mms_id\")  -> Ok, data_type = {}, len = {}", mms.data_type(), mms.len());

    // Option -> Result via anyhow::Context::with_context (lazy format!).
    let err = column(&batch, "indicator_id").unwrap_err();
    println!("column(\"indicator_id\") -> Err: {err}");
    println!();
    println!("  -> Lookup is by name, so column order on disk does not matter");
    println!("  -> The returned &ArrayRef borrows from the batch ('a lifetime)");
}

// ============================================================
// SECTION 2: String Column - Utf8 happy path vs an Int32 impostor
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_string_column() {
    println!("============================================================");
    println!("SECTION 2: String Column - Utf8 happy path vs an Int32 impostor");
    println!("============================================================");
    let batch = manifest_batch();
    let instruments = string_column(&batch, "instrument").expect("Utf8");
    for i in 0..instruments.len() {
        println!("  row {i}: {}", instruments.value(i));
    }

    // as_string_opt::<i32>() means Utf8 (32-bit offsets). A LargeUtf8 column
    // (i64 offsets) would ALSO be rejected here, despite holding strings.
    let err = string_column(&drifted_batch(), "instrument").unwrap_err();
    println!("drifted instrument -> Err: {err}");
    println!();
    println!("  -> Message names the column, not just the Arrow type");
}

// ============================================================
// SECTION 3: Primitive Width Drift - Int32 expected, Int64 on disk
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_primitive_width_drift() {
    println!("============================================================");
    println!("SECTION 3: Primitive Width Drift - Int32 expected, Int64 on disk");
    println!("============================================================");
    // The turbofish picks T: ArrowPrimitiveType; T::DATA_TYPE feeds the message.
    let manifest = manifest_batch();
    let drifted = drifted_batch();
    let ok = primitive_column::<Int32Type>(&manifest, "mms_id").expect("Int32");
    println!("manifest mms_id as Int32 -> Ok {:?}", ok.values());

    let err = primitive_column::<Int32Type>(&drifted, "mms_id").unwrap_err();
    println!("drifted  mms_id as Int32 -> Err: {err}");

    // Downcasting is exact: no widening/narrowing, even when values would fit.
    let wide = primitive_column::<Int64Type>(&drifted, "mms_id").expect("Int64");
    println!("drifted  mms_id as Int64 -> Ok {:?}", wide.values());
    println!();
    println!("  -> processed_manifest/strategy_adapter propagate this with `?`,");
    println!("     so a drifted file becomes an Err the caller reports, not an abort");
}

// ============================================================
// SECTION 4: Decimal Parameters - what primitive_column does NOT check
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_decimal_parameters() {
    println!("============================================================");
    println!("SECTION 4: Decimal Parameters - what primitive_column does NOT check");
    println!("============================================================");
    // strategy_adapter reads annual_roi etc. as Decimal128Type. The downcast
    // matches the *physical* array type, not the precision/scale parameters.
    let on_disk = Decimal128Array::from(vec![12_345_i128, -6_789])
        .with_precision_and_scale(20, 2)
        .expect("valid precision/scale");
    let batch = RecordBatch::try_from_iter([("annual_roi", Arc::new(on_disk) as ArrayRef)])
        .expect("well-formed");

    let rois = primitive_column::<Decimal128Type>(&batch, "annual_roi").expect("Decimal128");
    println!("on-disk type        : {}", rois.data_type());
    println!("T::DATA_TYPE (msg)  : {}", Decimal128Type::DATA_TYPE);
    println!("raw i128 values     : {:?}", rois.values());
    println!();
    println!("  -> Ok even though scale differs from the writer's METRIC_TYPE");
    println!("  -> Raw i128s are reinterpreted by the caller's own scale");
    println!("  -> Accepted by design (see commit 84354d5)");
}

fn int32_list(rows: &[&[i32]]) -> arrow::array::ListArray {
    let mut builder = ListBuilder::new(Int32Builder::new());
    for r in rows {
        builder.values().append_slice(r);
        builder.append(true);
    }
    builder.finish()
}

// ============================================================
// SECTION 5: List Items - list_column + list_items per row
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_list_items() {
    println!("============================================================");
    println!("SECTION 5: List Items - list_column + list_items per row");
    println!("============================================================");
    // Two-step check: list_column verifies the OUTER type (List, i32 offsets);
    // list_items verifies the INNER item type, per row.
    let ids = int32_list(&[&[101, 102, 103], &[], &[205]]);
    let batch = RecordBatch::try_from_iter([("long_operation_point_ids", Arc::new(ids) as ArrayRef)])
        .expect("well-formed");

    let list = list_column(&batch, "long_operation_point_ids").expect("List");
    for row in 0..list.len() {
        let items = list_items::<Int32Type>(list, row, "long_operation_point_ids").expect("Int32");
        println!("  row {row}: {items:?}");
    }
    println!();
    println!("  -> list.value(row) is a zero-copy slice; .to_vec() copies it out");
    println!("  -> Empty lists come back as an empty Vec, not an error");
    println!("  -> `name` is passed only to label the error message");
}

// ============================================================
// SECTION 6: List Items Caveats - mistyped items, nulls, row bounds
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_6_list_items_caveats() {
    println!("============================================================");
    println!("SECTION 6: List Items Caveats - mistyped items, nulls, row bounds");
    println!("============================================================");
    // Mistyped items: List<Int64> passes list_column, then fails in list_items.
    let mut wide = ListBuilder::new(Int64Builder::new());
    wide.values().append_slice(&[101, 102]);
    wide.append(true);
    let wide = wide.finish();
    let err = list_items::<Int32Type>(&wide, 0, "short_operation_point_ids").unwrap_err();
    println!("List<Int64> as Int32 -> Err: {err}");

    // Nulls: values() ignores the validity bitmap, so a null item surfaces as
    // the native default (0). The writer never emits null items, so this only
    // matters for foreign files.
    let mut nullable = ListBuilder::new(Int32Builder::new());
    nullable.values().append_value(7);
    nullable.values().append_null();
    nullable.append(true);
    let nullable = nullable.finish();
    let items = list_items::<Int32Type>(&nullable, 0, "ids").expect("Int32");
    println!("[7, null] read back  -> {items:?}  (null became 0)");

    // Row bounds are NOT checked: list.value(row) panics past the end.
    // Callers iterate 0..batch.num_rows(), which keeps `row` in range.
    println!("row bounds           -> unchecked; caller loops 0..num_rows()");
}

// ============================================================
// SECTION 7: Why Not as_primitive - the panic these helpers avoid
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_7_why_not_as_primitive() {
    println!("============================================================");
    println!("SECTION 7: Why Not as_primitive - the panic these helpers avoid");
    println!("============================================================");
    let batch = drifted_batch();
    let raw = batch.column_by_name("mms_id").expect("present").clone();

    // Silence the default panic hook so the output stays within this section.
    std::panic::set_hook(Box::new(|_| {}));
    let outcome =
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| raw.as_primitive::<Int32Type>().len()));
    let _ = std::panic::take_hook();
    println!("as_primitive::<Int32Type>()       -> panicked = {}", outcome.is_err());

    let checked = primitive_column::<Int32Type>(&batch, "mms_id");
    println!("primitive_column::<Int32Type>()   -> {:?}", checked.map(|a| a.len()).map_err(|e| e.to_string()));
    println!();
    println!("  -> In this example (dev profile) the panic unwinds and is caught");
    println!("  -> In release, panic = \"abort\": no catch_unwind, the whole batch");
    println!("     run (all rayon tasks) dies on the first drifted file");
    println!("  -> The helper returns Err; create_loop marks that cell failed for retry");
}
