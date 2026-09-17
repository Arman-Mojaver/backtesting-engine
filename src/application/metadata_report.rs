use std::collections::{HashMap, HashSet};
use std::fmt;

use crate::domain::cast;

/// DB-side state: what exists in the database.
///
/// The DB holds no (instrument, mms_id, indicator) relationship: indicators are
/// a flat catalog. The set of triples to process is IMPLIED — it is the
/// Cartesian product instruments x mms_ids, each cell running every indicator.
/// build_metadata_report forms that product internally.
pub struct DbMetadataState {
    pub instruments: Vec<String>,
    pub mms_ids: Vec<i32>,
    pub indicator_ids: Vec<i32>,
}

/// Manifest-side state: what has been processed.
pub struct ManifestMetadataState {
    /// Map of (instrument, mms_id) -> set of processed indicator_ids.
    pub processed_ids_per_cell: HashMap<(String, i32), HashSet<i32>>,
}

/// Per-cell (instrument + mms_id) breakdown row.
pub struct CellMetadata {
    pub instrument: String,
    pub mms_id: i32,
    pub db_indicator_count: i32,
    pub manifest_indicator_count: i32,
    pub remaining_indicator_ids: Vec<i32>,
}

/// Full metadata report.
pub struct MetadataReport {
    pub cells: Vec<CellMetadata>,
    pub total_db_indicators: i32,
    pub total_processed: i32,
    pub total_remaining: i32,
}

/// Builds a metadata report from DB and manifest state.
/// Pure function — no I/O, testable in isolation.
pub fn build_metadata_report(
    db_state: &DbMetadataState,
    manifest_state: &ManifestMetadataState,
) -> MetadataReport {
    // DB cells are the Cartesian product instruments x mms_ids.
    let mut db_cells: HashSet<(String, i32)> = HashSet::new();
    for instrument in &db_state.instruments {
        for mms_id in &db_state.mms_ids {
            db_cells.insert((instrument.clone(), *mms_id));
        }
    }

    // Collect all cells: the DB product plus any manifest-only cells.
    let mut all_cells = db_cells.clone();
    all_cells.extend(manifest_state.processed_ids_per_cell.keys().cloned());

    // The DB's indicator IDs as a set (for fast difference ops).
    let db_ids: HashSet<i32> = db_state.indicator_ids.iter().copied().collect();
    let db_count = db_state.indicator_ids.len();

    let mut cells: Vec<CellMetadata> = Vec::new();

    for (instrument, mms_id) in &all_cells {
        let manifest_ids = manifest_state
            .processed_ids_per_cell
            .get(&(instrument.clone(), *mms_id));

        let manifest_count = manifest_ids.map_or(0, |s| cast::usize_to_i32(s.len()));

        let (db_effective, remaining_ids) = if db_cells.contains(&(instrument.clone(), *mms_id)) {
            // Which indicator IDs from the DB have NOT been processed yet?
            let remaining: Vec<i32> = manifest_ids.map_or_else(
                || db_state.indicator_ids.clone(),
                |processed| db_ids.difference(processed).copied().collect(),
            );
            let mut r = remaining;
            r.sort_unstable();
            (cast::usize_to_i32(db_count), r)
        } else {
            // Manifest-only cell has no DB presence.
            (0, Vec::new())
        };

        cells.push(CellMetadata {
            instrument: instrument.clone(),
            mms_id: *mms_id,
            db_indicator_count: db_effective,
            manifest_indicator_count: manifest_count,
            remaining_indicator_ids: remaining_ids,
        });
    }

    // Sort by instrument then mms_id for stable output
    cells.sort_by(|a, b| {
        a.instrument
            .cmp(&b.instrument)
            .then_with(|| a.mms_id.cmp(&b.mms_id))
    });

    let total_db_indicators: i32 = cells.iter().map(|c| c.db_indicator_count).sum();
    let total_processed: i32 = cells.iter().map(|c| c.manifest_indicator_count).sum();
    let total_remaining: i32 = cells
        .iter()
        .map(|c| cast::usize_to_i32(c.remaining_indicator_ids.len()))
        .sum();

    MetadataReport {
        cells,
        total_db_indicators,
        total_processed,
        total_remaining,
    }
}

impl fmt::Display for MetadataReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Metadata Report")?;
        writeln!(f, "===============")?;
        writeln!(f)?;

        if self.cells.is_empty() {
            writeln!(f, "No data found.")?;
            return Ok(());
        }

        // Header
        writeln!(
            f,
            "{:<12} {:<8} {:<14} {:<14} {:<12}",
            "Instrument", "MMS ID", "DB Indicators", "Manifest", "Remaining"
        )?;
        writeln!(f, "{}", "-".repeat(60))?;

        // Rows
        for cell in &self.cells {
            writeln!(
                f,
                "{:<12} {:<8} {:<14} {:<14} {:<12}",
                cell.instrument,
                cell.mms_id,
                cell.db_indicator_count,
                cell.manifest_indicator_count,
                cast::usize_to_i32(cell.remaining_indicator_ids.len()),
            )?;
        }

        writeln!(f, "{}", "-".repeat(60))?;
        writeln!(
            f,
            "{:<21} {:<14} {:<14} {:<12}",
            "TOTALS:", self.total_db_indicators, self.total_processed, self.total_remaining,
        )?;
        writeln!(f)?;

        if self.total_remaining == 0 {
            writeln!(f, "All triples are processed. Nothing to do.")?;
        } else {
            writeln!(f, "{} triples remaining to process.", self.total_remaining)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(range: std::ops::RangeInclusive<i32>) -> Vec<i32> {
        range.collect()
    }

    fn empty_db_state() -> DbMetadataState {
        DbMetadataState {
            instruments: vec![],
            mms_ids: vec![],
            indicator_ids: vec![],
        }
    }

    fn empty_manifest_state() -> ManifestMetadataState {
        ManifestMetadataState {
            processed_ids_per_cell: HashMap::new(),
        }
    }

    #[test]
    fn empty_db_and_manifest_produces_empty_report() {
        let db = empty_db_state();
        let manifest = empty_manifest_state();
        let report = build_metadata_report(&db, &manifest);
        assert!(report.cells.is_empty());
        assert_eq!(report.total_db_indicators, 0);
        assert_eq!(report.total_processed, 0);
        assert_eq!(report.total_remaining, 0);
    }

    #[test]
    fn db_cells_are_the_cartesian_product() {
        let db = DbMetadataState {
            instruments: vec!["EURUSD".to_string(), "USDJPY".to_string()],
            mms_ids: vec![1, 2],
            indicator_ids: ids(1..=4),
        };
        let manifest = empty_manifest_state();
        let report = build_metadata_report(&db, &manifest);

        // 2 instruments x 2 mms_ids = 4 cells, each seeing all 4 indicators
        assert_eq!(report.cells.len(), 4);
        assert!(report.cells.iter().all(|c| c.db_indicator_count == 4));
        assert_eq!(report.total_db_indicators, 16);
        assert_eq!(report.total_remaining, 16);
        // No manifest yet — every indicator remains
        for cell in &report.cells {
            assert_eq!(cell.remaining_indicator_ids, vec![1, 2, 3, 4]);
        }
    }

    #[test]
    fn db_only_with_no_manifest_shows_all_remaining() {
        let db = DbMetadataState {
            instruments: vec!["EURUSD".to_string()],
            mms_ids: vec![1, 2],
            indicator_ids: ids(1..=10),
        };
        let manifest = empty_manifest_state();
        let report = build_metadata_report(&db, &manifest);

        assert_eq!(report.cells.len(), 2);
        assert_eq!(report.total_db_indicators, 20);
        assert_eq!(report.total_processed, 0);
        assert_eq!(report.total_remaining, 20);

        let cell1 = &report.cells[0];
        assert_eq!(cell1.instrument, "EURUSD");
        assert_eq!(cell1.mms_id, 1);
        assert_eq!(cell1.db_indicator_count, 10);
        assert_eq!(cell1.manifest_indicator_count, 0);
        assert_eq!(cell1.remaining_indicator_ids.len(), 10);
        assert_eq!(cell1.remaining_indicator_ids, ids(1..=10));
    }

    #[test]
    fn fully_processed_shows_zero_remaining() {
        let db = DbMetadataState {
            instruments: vec!["EURUSD".to_string()],
            mms_ids: vec![1],
            indicator_ids: vec![1, 2, 3],
        };
        let manifest = ManifestMetadataState {
            processed_ids_per_cell: HashMap::from([(
                ("EURUSD".to_string(), 1),
                HashSet::from([1, 2, 3]),
            )]),
        };
        let report = build_metadata_report(&db, &manifest);

        assert_eq!(report.cells.len(), 1);
        let cell = &report.cells[0];
        assert_eq!(cell.db_indicator_count, 3);
        assert_eq!(cell.manifest_indicator_count, 3);
        assert_eq!(cell.remaining_indicator_ids.len(), 0);
        assert!(cell.remaining_indicator_ids.is_empty());
        assert_eq!(report.total_remaining, 0);
    }

    #[test]
    fn partially_processed_shows_correct_remaining() {
        let db = DbMetadataState {
            instruments: vec!["EURUSD".to_string()],
            mms_ids: vec![1],
            indicator_ids: ids(1..=5),
        };
        let manifest = ManifestMetadataState {
            processed_ids_per_cell: HashMap::from([(
                ("EURUSD".to_string(), 1),
                HashSet::from([1, 3]),
            )]),
        };
        let report = build_metadata_report(&db, &manifest);

        assert_eq!(report.total_db_indicators, 5);
        assert_eq!(report.total_processed, 2);
        assert_eq!(report.total_remaining, 3);

        let cell = &report.cells[0];
        assert_eq!(cell.remaining_indicator_ids, vec![2, 4, 5]);
    }

    #[test]
    fn manifest_ids_not_in_db_are_ignored_in_set_difference() {
        let db = DbMetadataState {
            instruments: vec!["EURUSD".to_string()],
            mms_ids: vec![1],
            indicator_ids: ids(1..=3),
        };
        let manifest = ManifestMetadataState {
            processed_ids_per_cell: HashMap::from([
                // id 99 exists in manifest but not in DB — difference ignores it
                (("EURUSD".to_string(), 1), HashSet::from([1, 99])),
            ]),
        };
        let report = build_metadata_report(&db, &manifest);

        let cell = &report.cells[0];
        assert_eq!(cell.manifest_indicator_count, 2); // raw count = 2
        assert_eq!(cell.remaining_indicator_ids.len(), 2); // db has 1,2,3 minus processed {1} = {2,3}
        assert_eq!(cell.remaining_indicator_ids, vec![2, 3]);
    }

    #[test]
    fn manifest_only_cell_included_with_zero_db_count() {
        let db = empty_db_state();
        let manifest = ManifestMetadataState {
            processed_ids_per_cell: HashMap::from([(
                ("EURUSD".to_string(), 1),
                HashSet::from([1]),
            )]),
        };
        let report = build_metadata_report(&db, &manifest);

        assert_eq!(report.cells.len(), 1);
        let cell = &report.cells[0];
        assert_eq!(cell.instrument, "EURUSD");
        assert_eq!(cell.mms_id, 1);
        assert_eq!(cell.db_indicator_count, 0);
        assert_eq!(cell.manifest_indicator_count, 1);
        assert_eq!(cell.remaining_indicator_ids.len(), 0);
        assert!(cell.remaining_indicator_ids.is_empty());
    }

    #[test]
    fn display_format_outputs_table() {
        let db = DbMetadataState {
            instruments: vec!["EURUSD".to_string()],
            mms_ids: vec![1],
            indicator_ids: ids(1..=10),
        };
        let manifest = empty_manifest_state();
        let report = build_metadata_report(&db, &manifest);
        let output = format!("{report}");

        assert!(output.contains("Metadata Report"));
        assert!(output.contains("Instrument"));
        assert!(output.contains("EURUSD"));
        assert!(output.contains("10"));
        assert!(output.contains("remaining"));
    }

    #[test]
    fn display_empty_report_shows_no_data() {
        let db = empty_db_state();
        let manifest = empty_manifest_state();
        let report = build_metadata_report(&db, &manifest);
        let output = format!("{report}");
        assert!(output.contains("No data found"));
    }

    #[test]
    fn display_fully_processed_shows_nothing_to_do() {
        let db = DbMetadataState {
            instruments: vec!["EURUSD".to_string()],
            mms_ids: vec![1],
            indicator_ids: vec![1, 2, 3],
        };
        let manifest = ManifestMetadataState {
            processed_ids_per_cell: HashMap::from([(
                ("EURUSD".to_string(), 1),
                HashSet::from([1, 2, 3]),
            )]),
        };
        let report = build_metadata_report(&db, &manifest);
        let output = format!("{report}");
        assert!(output.contains("All triples are processed"));
    }

    #[test]
    fn report_cells_are_sorted_by_instrument_then_mms_id() {
        let db = DbMetadataState {
            instruments: vec!["USDJPY".to_string(), "EURUSD".to_string()],
            mms_ids: vec![9, 1],
            indicator_ids: vec![42],
        };
        let manifest = empty_manifest_state();
        let report = build_metadata_report(&db, &manifest);

        assert_eq!(report.cells.len(), 4);
        // Verify sort order: EURUSD before USDJPY, then mms_id ascending
        let order: Vec<(String, i32)> = report
            .cells
            .iter()
            .map(|c| (c.instrument.clone(), c.mms_id))
            .collect();
        assert_eq!(
            order,
            vec![
                ("EURUSD".to_string(), 1),
                ("EURUSD".to_string(), 9),
                ("USDJPY".to_string(), 1),
                ("USDJPY".to_string(), 9),
            ]
        );
    }
}
