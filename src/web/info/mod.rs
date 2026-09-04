//! Pure stat-assembly types and functions (no I/O — unit-testable).
//!
//! The adapters collect raw data; this module shapes it into display-ready
//! view models. Everything here is deterministic and covered by unit tests.

use std::collections::{HashMap, HashSet};

use crate::application::metadata_report::{
    build_metadata_report, DbMetadataState, ManifestMetadataState, MetadataReport,
};

/// Everything the main page renders.
pub struct SystemOverview {
    pub environment: String,
    pub generated_at: String,
    pub host: HostInfo,
    pub database: DatabaseInfo,
    pub processing: ProcessingInfo,
}

/// Host/machine section: live CPU and memory usage only.
pub struct HostInfo {
    /// Mean usage across all CPUs since the previous sample.
    pub cpu_usage_percent: f32,
    pub memory_total_bytes: u64,
    pub memory_used_bytes: u64,
    /// `used / total` in percent.
    pub memory_percent: f32,
}

/// Database section.
pub struct DatabaseInfo {
    pub connected: bool,
    /// Empty when connected; the failure message otherwise.
    pub error: String,
    /// Human-readable target (e.g. `db-development @ host:54320`).
    pub target: String,
    pub table_counts: Vec<TableCount>,
    pub instruments: Vec<InstrumentStats>,
}

impl DatabaseInfo {
    /// The database section shown when the DB is unreachable.
    pub fn unreachable(target: &str, error: &str) -> Self {
        Self {
            connected: false,
            error: error.to_string(),
            target: target.to_string(),
            table_counts: Vec::new(),
            instruments: Vec::new(),
        }
    }
}

/// One row in the table-count list.
pub struct TableCount {
    pub table: &'static str,
    pub rows: i64,
}

/// One row in the instruments list.
pub struct InstrumentStats {
    pub instrument: String,
    pub points: i64,
    /// ISO date of the first point.
    pub first_date: String,
    /// ISO date of the last point.
    pub last_date: String,
}

/// Processing/parquet section.
pub struct ProcessingInfo {
    /// Empty when the parquet tree was readable; the failure message otherwise.
    pub error: String,
    pub manifest_entries: usize,
    pub manifest_instruments: usize,
    pub manifest_mms_ids: usize,
    /// Whether DB + manifest were both available to compute progress.
    pub progress_available: bool,
    pub progress_total: i64,
    pub progress_processed: i64,
    pub progress_remaining: i64,
    /// Processed/total in percent (0.0 when unavailable).
    pub progress_percent: f32,
    /// Number of (instrument x mms) partitions with strategy output.
    pub strategy_partitions: usize,
    pub total_strategies: i64,
    pub parquet_bytes: u64,
}

impl ProcessingInfo {
    /// The processing section shown when the parquet tree is unreadable.
    pub fn unavailable(error: String) -> Self {
        Self {
            error,
            manifest_entries: 0,
            manifest_instruments: 0,
            manifest_mms_ids: 0,
            progress_available: false,
            progress_total: 0,
            progress_processed: 0,
            progress_remaining: 0,
            progress_percent: 0.0,
            strategy_partitions: 0,
            total_strategies: 0,
            parquet_bytes: 0,
        }
    }
}

/// Raw parquet scan output (collected behind `spawn_blocking`).
pub struct ParquetScan {
    /// Map of (instrument, mms_id) -> processed indicator ids.
    pub processed_ids_per_cell: HashMap<(String, i32), HashSet<i32>>,
    pub strategy_partitions: usize,
    pub total_strategies: i64,
    pub parquet_bytes: u64,
}

/// Formats a Unix epoch (seconds) as an ISO date (`YYYY-MM-DD`).
pub fn format_epoch_date(epoch_seconds: i32) -> String {
    match time::OffsetDateTime::from_unix_timestamp(i64::from(epoch_seconds)) {
        Ok(datetime) => datetime.date().to_string(),
        Err(_) => "unknown".to_string(),
    }
}

/// `part / total` in percent (0.0 when total is zero).
pub fn percent(part: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    part as f32 / total as f32 * 100.0
}

/// Assembles the processing section from DB metadata and a parquet scan.
///
/// Pure: combines the (optional) DB metadata state with the manifest scan
/// through [`build_metadata_report`]. `db` is `None` when the DB is
/// unreachable — progress is then reported as unavailable.
pub fn build_processing(db: Option<&DbMetadataState>, scan: ParquetScan) -> ProcessingInfo {
    let ParquetScan {
        processed_ids_per_cell,
        strategy_partitions,
        total_strategies,
        parquet_bytes,
    } = scan;

    let manifest_entries: usize = processed_ids_per_cell.values().map(HashSet::len).sum();
    let manifest_instruments = processed_ids_per_cell
        .keys()
        .map(|(instrument, _)| instrument)
        .collect::<HashSet<_>>()
        .len();
    let manifest_mms_ids = processed_ids_per_cell
        .keys()
        .map(|(_, mms_id)| mms_id)
        .collect::<HashSet<_>>()
        .len();

    let progress = db.map(|db_state| {
        let manifest_state = ManifestMetadataState {
            processed_ids_per_cell,
        };
        let report = build_metadata_report(db_state, &manifest_state);
        progress_from_report(&report)
    });

    let (available, total, processed, remaining, ratio) = match progress {
        Some(numbers) => (
            true,
            numbers.total,
            numbers.processed,
            numbers.remaining,
            numbers.percent,
        ),
        None => (false, 0, 0, 0, 0.0),
    };

    ProcessingInfo {
        error: String::new(),
        manifest_entries,
        manifest_instruments,
        manifest_mms_ids,
        progress_available: available,
        progress_total: total,
        progress_processed: processed,
        progress_remaining: remaining,
        progress_percent: ratio,
        strategy_partitions,
        total_strategies,
        parquet_bytes,
    }
}

/// Headline progress numbers extracted from a metadata report.
struct ProgressNumbers {
    total: i64,
    processed: i64,
    remaining: i64,
    percent: f32,
}

fn progress_from_report(report: &MetadataReport) -> ProgressNumbers {
    let total = i64::from(report.total_db_indicators);
    let processed = i64::from(report.total_processed);
    let remaining = i64::from(report.total_remaining);
    let ratio = if total > 0 {
        processed as f32 / total as f32 * 100.0
    } else {
        0.0
    };
    ProgressNumbers {
        total,
        processed,
        remaining,
        percent: ratio,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_with(cells: HashMap<(String, i32), HashSet<i32>>) -> ParquetScan {
        ParquetScan {
            processed_ids_per_cell: cells,
            strategy_partitions: 3,
            total_strategies: 42,
            parquet_bytes: 4_900_000,
        }
    }

    fn db_state() -> DbMetadataState {
        DbMetadataState {
            instruments: vec!["EURUSD".to_string()],
            mms_ids: vec![1, 2],
            indicator_ids: vec![1, 2, 3],
        }
    }

    #[test]
    fn format_epoch_date_renders_epoch_start() {
        assert_eq!(format_epoch_date(0), "1970-01-01");
    }

    #[test]
    fn format_epoch_date_renders_known_date() {
        // First resampled_point_d1 timestamp in the development DB.
        assert_eq!(format_epoch_date(1_354_579_200), "2012-12-04");
    }

    #[test]
    fn percent_computes_ratio() {
        assert!((percent(1, 4) - 25.0).abs() < f32::EPSILON);
    }

    #[test]
    fn percent_zero_total_is_zero() {
        assert_eq!(percent(5, 0), 0.0);
    }

    #[test]
    fn build_processing_combines_db_and_manifest() {
        let cells = HashMap::from([(("EURUSD".to_string(), 1), HashSet::from([1]))]);
        let info = build_processing(Some(&db_state()), scan_with(cells));

        // 2 cells x 3 indicators = 6 total; 1 processed; 5 remaining.
        assert_eq!(info.error, "");
        assert_eq!(info.manifest_entries, 1);
        assert_eq!(info.manifest_instruments, 1);
        assert_eq!(info.manifest_mms_ids, 1);
        assert!(info.progress_available);
        assert_eq!(info.progress_total, 6);
        assert_eq!(info.progress_processed, 1);
        assert_eq!(info.progress_remaining, 5);
        assert!((info.progress_percent - 100.0 / 6.0).abs() < 0.01);
        assert_eq!(info.strategy_partitions, 3);
        assert_eq!(info.total_strategies, 42);
        assert_eq!(info.parquet_bytes, 4_900_000);
    }

    #[test]
    fn build_processing_without_db_reports_progress_unavailable() {
        let cells = HashMap::from([(("EURUSD".to_string(), 1), HashSet::from([1]))]);
        let info = build_processing(None, scan_with(cells));

        assert!(!info.progress_available);
        assert_eq!(info.progress_total, 0);
        assert_eq!(info.progress_percent, 0.0);
        // Manifest stats are still shown.
        assert_eq!(info.manifest_entries, 1);
    }

    #[test]
    fn build_processing_with_empty_manifest_shows_all_remaining() {
        let info = build_processing(Some(&db_state()), scan_with(HashMap::new()));

        assert!(info.progress_available);
        assert_eq!(info.manifest_entries, 0);
        assert_eq!(info.progress_total, 6);
        assert_eq!(info.progress_processed, 0);
        assert_eq!(info.progress_remaining, 6);
        assert_eq!(info.progress_percent, 0.0);
    }

    #[test]
    fn build_processing_counts_manifest_dimensions() {
        let cells = HashMap::from([
            (("EURUSD".to_string(), 1), HashSet::from([1])),
            (("EURUSD".to_string(), 2), HashSet::from([1, 2])),
            (("USDJPY".to_string(), 1), HashSet::from([3])),
        ]);
        let info = build_processing(None, scan_with(cells));

        assert_eq!(info.manifest_entries, 4);
        assert_eq!(info.manifest_instruments, 2);
        assert_eq!(info.manifest_mms_ids, 2);
    }
}
