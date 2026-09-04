//! Application state: shared dependencies injected into every handler.

use std::str::FromStr;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod};
use log::{info, warn};
use sysinfo::System;

use crate::config::environment::{parquet_root_dir, DbConfig, Environment};

/// Shared state injected into every handler via `Router::with_state`.
///
/// Immutable after construction; cloning is cheap (pool + `Arc` internals).
#[derive(Clone)]
pub struct AppState {
    /// Async connection pool (webapp-owned; the engine's sync `postgres`
    /// adapters are untouched and used only by `bt-engine`).
    pub pool: Pool,
    /// Active environment (selects DB target + parquet subtree).
    pub environment: Environment,
    /// Human-readable environment label for the page badge.
    pub environment_label: String,
    /// Human-readable DB target shown on the page.
    pub db_target: String,
    /// Parquet root for the active environment (manifest + strategy tree).
    pub parquet_root: String,
    /// Long-lived sysinfo handle. CPU usage is diff-based, so a single
    /// `System` is kept for the process lifetime and refreshed per request.
    pub sys: Arc<Mutex<System>>,
}

/// Reads `ENVIRONMENT` (default `development`), mirroring the engine binaries.
pub fn resolve_environment() -> Environment {
    std::env::var("ENVIRONMENT")
        .ok()
        .and_then(|raw| Environment::from_str(&raw).ok())
        .unwrap_or(Environment::Development)
}

/// Builds the pool and state for `environment`.
pub async fn build_state(environment: Environment) -> Result<AppState> {
    let db_config = DbConfig::from_env(environment);
    let db_target = format!(
        "{database} @ {host}:{port}",
        database = db_config.database,
        host = db_config.host,
        port = db_config.port,
    );

    let pg_config = tokio_postgres::Config::from_str(&db_config.connection_string())
        .context("invalid DB connection string")?;
    let manager = Manager::from_config(
        pg_config,
        tokio_postgres::NoTls,
        ManagerConfig {
            recycling_method: RecyclingMethod::Fast,
        },
    );
    let pool = Pool::builder(manager)
        .max_size(4)
        .build()
        .context("failed to build DB pool")?;

    // Warm-up: verify the DB is reachable and log the outcome. The server
    // starts regardless — the page degrades gracefully when the DB is down.
    match pool.get().await {
        Ok(client) if client.simple_query("SELECT 1").await.is_ok() => {
            info!("DB reachable: {db_target}");
        }
        outcome => warn!("DB unreachable at startup ({db_target}): {outcome:?}"),
    }

    // Seed the CPU-usage baseline so the first request already reports a
    // meaningful value (usage is computed between two refresh samples).
    let mut sys = System::new();
    sys.refresh_cpu_usage();
    sys.refresh_memory();

    Ok(AppState {
        pool,
        environment,
        environment_label: environment_label(environment).to_string(),
        db_target,
        parquet_root: parquet_root_dir(environment),
        sys: Arc::new(Mutex::new(sys)),
    })
}

/// Maps an [`Environment`] to its display label.
fn environment_label(environment: Environment) -> &'static str {
    match environment {
        Environment::Production => "production",
        Environment::Development => "development",
        Environment::Testing => "testing",
    }
}
