//! Async DB stats adapter (webapp-owned).
//!
//! Queries only aggregates — the engine's sync entity repositories are
//! untouched and remain the sole data path of `bt-engine`. All queries are
//! issued concurrently on a single pooled client: tokio-postgres pipelines
//! them over one connection.

use deadpool_postgres::Pool;
use tokio_postgres::Row;

use crate::application::metadata_report::DbMetadataState;
use crate::web::info::{format_epoch_date, DatabaseInfo, InstrumentStats, TableCount};

/// Everything the page needs from the DB in one round.
pub struct DbSnapshot {
    pub info: DatabaseInfo,
    pub metadata: DbMetadataState,
}

/// Failure while collecting DB stats (rendered inline on the page, not a 500).
#[derive(Debug, thiserror::Error)]
pub enum DbCollectError {
    #[error("{0}")]
    Pool(#[from] deadpool_postgres::PoolError),
    #[error("{0}")]
    Query(#[from] tokio_postgres::Error),
}

/// Collects all DB stats concurrently on one pooled client.
pub async fn collect(pool: &Pool, target: &str) -> Result<DbSnapshot, DbCollectError> {
    let client = pool.get().await?;

    // Every query future is created before the first `.await` —
    // tokio-postgres pipelines them over the single connection.
    let (
        indicator_count,
        mms_count,
        long_count,
        short_count,
        resampled_count,
        raw_d1_count,
        raw_h1_count,
        instrument_rows,
        indicator_id_rows,
        mms_id_rows,
    ) = tokio::join!(
        count(&client, "SELECT count(*) FROM indicator"),
        count(&client, "SELECT count(*) FROM money_management_strategy"),
        count(&client, "SELECT count(*) FROM long_operation_point"),
        count(&client, "SELECT count(*) FROM short_operation_point"),
        count(&client, "SELECT count(*) FROM resampled_point_d1"),
        count(&client, "SELECT count(*) FROM raw_point_d1"),
        count(&client, "SELECT count(*) FROM raw_point_h1"),
        client.query(
            "SELECT instrument, count(*), min(timestamp), max(timestamp) \
             FROM resampled_point_d1 \
             GROUP BY instrument \
             ORDER BY instrument",
            &[],
        ),
        client.query("SELECT id FROM indicator ORDER BY id", &[]),
        client.query(
            "SELECT DISTINCT money_management_strategy_id \
             FROM long_operation_point \
             ORDER BY money_management_strategy_id",
            &[],
        ),
    );

    let table_counts = vec![
        TableCount {
            table: "indicator",
            rows: indicator_count?,
        },
        TableCount {
            table: "money_management_strategy",
            rows: mms_count?,
        },
        TableCount {
            table: "long_operation_point",
            rows: long_count?,
        },
        TableCount {
            table: "short_operation_point",
            rows: short_count?,
        },
        TableCount {
            table: "resampled_point_d1",
            rows: resampled_count?,
        },
        TableCount {
            table: "raw_point_d1",
            rows: raw_d1_count?,
        },
        TableCount {
            table: "raw_point_h1",
            rows: raw_h1_count?,
        },
    ];

    let (instruments, metadata_instruments) = instrument_stats(&instrument_rows?)?;
    let indicator_ids = i32_column(&indicator_id_rows?, 0)?;
    let mms_ids = i32_column(&mms_id_rows?, 0)?;

    Ok(DbSnapshot {
        info: DatabaseInfo {
            connected: true,
            error: String::new(),
            target: target.to_string(),
            table_counts,
            instruments,
        },
        metadata: DbMetadataState {
            instruments: metadata_instruments,
            mms_ids,
            indicator_ids,
        },
    })
}

/// Runs a single-value `SELECT count(*)`-style query.
async fn count(
    client: &deadpool_postgres::Client,
    sql: &'static str,
) -> Result<i64, tokio_postgres::Error> {
    let row = client.query_one(sql, &[]).await?;
    row.try_get(0)
}

/// Builds the instrument table rows plus the distinct instrument list
/// (one GROUP BY query feeds both).
fn instrument_stats(
    rows: &[Row],
) -> Result<(Vec<InstrumentStats>, Vec<String>), tokio_postgres::Error> {
    let mut stats = Vec::with_capacity(rows.len());
    let mut names = Vec::with_capacity(rows.len());
    for row in rows {
        let instrument: String = row.try_get(0)?;
        let points: i64 = row.try_get(1)?;
        let first: i32 = row.try_get(2)?;
        let last: i32 = row.try_get(3)?;
        names.push(instrument.clone());
        stats.push(InstrumentStats {
            instrument,
            points,
            first_date: format_epoch_date(first),
            last_date: format_epoch_date(last),
        });
    }
    Ok((stats, names))
}

/// Extracts a single i32 column from all rows.
fn i32_column(rows: &[Row], column: usize) -> Result<Vec<i32>, tokio_postgres::Error> {
    rows.iter()
        .map(|row| row.try_get::<usize, i32>(column))
        .collect()
}
