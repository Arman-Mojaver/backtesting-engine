//! Test-only fixture: fetch real EURUSD resampled points from the
//! development database. Returns `None` (skip) when the DB is unavailable
//! so the test suite stays runnable in environments without the DB.
//!
//! Connects via `localhost` (host-resolvable) rather than
//! `host.docker.internal` (container-resolvable only), per PRD Decision 6.

use postgres::{Client, NoTls, Row};

use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};

/// Development DB on `localhost:54320` (host-resolvable).
const DEV_DB_URL: &str = "postgresql://postgres:postgres@localhost:54320/db-development";
const INSTRUMENT: &str = "EURUSD";

fn map_high_low_order(value: &str) -> HighLowOrder {
    match value {
        "high_first" => HighLowOrder::HighFirst,
        "low_first" => HighLowOrder::LowFirst,
        _ => HighLowOrder::Undefined,
    }
}

fn row_to_resampled_point(row: &Row) -> ResampledPointD1 {
    ResampledPointD1 {
        id: row.get("id"),
        instrument: row.get("instrument"),
        open: row.get("open"),
        high: row.get("high"),
        low: row.get("low"),
        close: row.get("close"),
        volume: row.get("volume"),
        timestamp: row.get::<_, i32>("timestamp") as i64,
        high_low_order: map_high_low_order(&row.get::<_, String>("high_low_order")),
    }
}

/// Fetch real EURUSD `resampled_point_d1` rows from the development DB,
/// ordered by timestamp. Returns `None` if the DB is unreachable so callers
/// can skip (not fail) gracefully.
pub fn fetch_eurusd_points() -> Option<Vec<ResampledPointD1>> {
    let mut client = Client::connect(DEV_DB_URL, NoTls).ok()?;
    let rows = client
        .query(
            "SELECT id, instrument, open, high, low, close, volume, timestamp, high_low_order \
             FROM resampled_point_d1 WHERE instrument = $1 ORDER BY timestamp",
            &[&INSTRUMENT],
        )
        .ok()?;
    Some(rows.iter().map(row_to_resampled_point).collect())
}
