//! Deep Dive: pg_adapter (2/2) — PgStore, the repository ports and row mapping
//! Source: Code input — src/infrastructure/pg_adapter.rs
//!
//! PgStore wraps one synchronous postgres::Client and implements all four
//! read ports the domain defines. Each query runs through fetch_rows, which
//! streams rows and maps them one by one with a row_to_* function.
//!
//! No database is needed. Section 1 makes a real connect attempt, but to
//! 127.0.0.1:1, where nothing listens, so it fails at once and never reaches a
//! server. The other sections print the SQL and the commands to try against
//! your own dev database; nothing here runs them.

use std::env;

use backtesting_engine::config::environment::{DbConfig, Environment};
use backtesting_engine::infrastructure::pg_adapter::PgStore;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str).unwrap_or("list") {
        "1" => section_1_connect_failure(),
        "2" => section_2_connection_settings(),
        "3" => section_3_ports_and_sql(),
        "4" => section_4_row_mapping_guards(),
        "5" => section_5_fetch_rows_streaming(),
        _ => list_sections(),
    }
}

fn list_sections() {
    println!("Usage: cargo run --example deep-dive-20261119-pg-adapter-connection <section>");
    println!();
    println!("  list - Show this list");
    println!("  1    - Connect Failure: the diagnostics in the error chain");
    println!("  2    - Connection Settings: DbConfig per environment, timeout, app name");
    println!("  3    - Ports And SQL: one PgStore, four repository traits");
    println!("  4    - Row Mapping Guards: sl <= 0, i32 timestamps, column context");
    println!("  5    - fetch_rows Streaming: query_raw, fail-fast mapping, debug timing");
}

// ============================================================
// SECTION 1: Connect Failure - the diagnostics in the error chain
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_1_connect_failure() {
    println!("============================================================");
    println!("SECTION 1: Connect Failure - the diagnostics in the error chain");
    println!("============================================================");
    let config = DbConfig {
        host: "127.0.0.1".into(),
        port: 1,
        user: "engine_role".into(),
        password: "s3cret-pw".into(),
        database: "db-development".into(),
    };
    let err = PgStore::connect(&config).unwrap_err();
    for (depth, cause) in err.chain().enumerate() {
        println!("[{depth}] {cause}");
    }
    println!();
    println!("password in message? {}", format!("{err:#}").contains("s3cret-pw"));
    println!("  -> connection_diagnostics is the context; the driver error is the cause");
    println!("  -> with_context is lazy: the text is only built when connect fails");
}

// ============================================================
// SECTION 2: Connection Settings - DbConfig per environment, timeout, app name
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_2_connection_settings() {
    println!("============================================================");
    println!("SECTION 2: Connection Settings - DbConfig per environment, timeout, app name");
    println!("============================================================");
    for env in [Environment::Development, Environment::Testing, Environment::Production] {
        let c = DbConfig::from_env(env);
        println!("{:<11} -> {}:{} / {}", env.as_str(), c.host, c.port, c.database);
    }
    println!("Debug: {:?}", DbConfig::from_env(Environment::Development));
    println!();
    println!("Fixed by PgStore::connect:");
    println!("  connect_timeout  = 10s   (CONNECT_TIMEOUT; an unreachable host fails, not hangs)");
    println!("  application_name = bte   (labels the session server-side)");
    println!("  TLS              = NoTls (local containers only)");
    println!();
    println!("While a run is active, see its sessions with:");
    println!("  psql -h localhost -p 54320 -U postgres -d db-development \\");
    println!("    -c \"SELECT pid, state, query FROM pg_stat_activity WHERE application_name = 'bte'\"");
    println!("  -> `strategy create` shows 1 session; `strategy create_loop` shows 4");
}

// ============================================================
// SECTION 3: Ports And SQL - one PgStore, four repository traits
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_3_ports_and_sql() {
    println!("============================================================");
    println!("SECTION 3: Ports And SQL - one PgStore, four repository traits");
    println!("============================================================");
    println!("IndicatorRepository::get_all");
    println!("  SELECT id, type, parameters::text FROM indicator ORDER BY id");
    println!("ResampledPointD1Repository::get_all / get_by_instrument($1)");
    println!("  SELECT id, instrument, open, ..., timestamp FROM resampled_point_d1");
    println!("  [WHERE instrument = $1] ORDER BY timestamp      <- load-bearing");
    println!("Long/ShortOperationPointRepository::get_all");
    println!("  SELECT ... FROM {{long,short}}_operation_point");
    println!("  ORDER BY instrument, money_management_strategy_id");
    println!("Long/ShortOperationPointRepository::get_by_mms_id_and_instrument($1, $2)");
    println!("  ... WHERE money_management_strategy_id = $1 AND instrument = $2");
    println!();
    println!("  -> Long and Short share a method name, so callers use");
    println!("     LongOperationPointRepository::get_all(&mut store) syntax");
    println!("  -> parameters::text: the column is json; reading it as text keeps");
    println!("     postgres' serde_json feature out and hands parse_params a &str");
    println!("  -> create: one PgStore, reads in sequence (source_data.rs)");
    println!("  -> create_loop: four PgStores, loads in rayon::join (strategy_batch.rs)");
}

// ============================================================
// SECTION 4: Row Mapping Guards - sl <= 0, i32 timestamps, column context
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_4_row_mapping_guards() {
    println!("============================================================");
    println!("SECTION 4: Row Mapping Guards - sl <= 0, i32 timestamps, column context");
    println!("============================================================");
    println!("row_to_operation_point rejects sl <= 0:");
    println!("  error: operation point id 812 (EURUSD) has non-positive sl 0");
    println!("  why:   performance metrics divide by sl; 0 gives inf/NaN and the");
    println!("         strategy would be dropped downstream without a trace");
    println!();
    let ts: i32 = 1_731_974_400; // 2024-11-19, as stored in the int4 column
    println!("timestamp: column::<i32> then i64::from: {ts}_i32 -> {}_i64", i64::from(ts));
    println!("  the DB column is int4; the domain uses i64 for timestamp arithmetic");
    println!();
    println!("Every read goes through column(row, table, name), so a schema drift reads:");
    println!("  failed to read column long_operation_point.risk");
    println!("  caused by: error deserializing column 5: cannot convert between ...");
    println!();
    println!("Find rows that would trip the sl guard in your dev DB:");
    println!("  psql -h localhost -p 54320 -U postgres -d db-development \\");
    println!("    -c \"SELECT id, instrument, sl FROM long_operation_point WHERE sl <= 0\"");
}

// ============================================================
// SECTION 5: fetch_rows Streaming - query_raw, fail-fast mapping, debug timing
// (Copy the title above and search this file to find this code!)
// ============================================================
fn section_5_fetch_rows_streaming() {
    println!("============================================================");
    println!("SECTION 5: fetch_rows Streaming - query_raw, fail-fast mapping, debug timing");
    println!("============================================================");
    println!("fetch_rows(conn, label, sql, params, map):");
    println!("  1. conn.query_raw(sql, params)  -> a lazy RowIter, not a Vec<Row>");
    println!("  2. while let Some(row) = rows.next()?  -> one Row at a time");
    println!("  3. out.push(map(&row)?)  -> first bad row aborts the whole query");
    println!("  4. debug!(\"query fetched and mapped ({{label}}): N rows in D\")");
    println!();
    println!("  -> Only one Row is alive at a time, so the full Vec<Row> is never");
    println!("     held next to the mapped Vec<T>");
    println!("  -> `label` appears in both the error context and the log line,");
    println!("     so either one tells you which of the 7 queries it was");
    println!();
    println!("See the timings yourself (needs the dev DB up):");
    println!("  RUST_LOG=debug bte strategy create -m 390 -i EURUSD");
    println!();
    println!("Lines of this shape on stderr / logs/bte.log (counts illustrative):");
    println!("  query fetched and mapped (indicator get_all): 412 rows in 3.1ms");
    println!("  query fetched and mapped (resampled_point_d1 by instrument): 5210 rows in 9.8ms");
}
