//! Data collectors.
//!
//! - [`db`]: webapp-owned async adapter (tokio-postgres pool)
//! - [`host`]: sysinfo sampling on the blocking pool
//! - [`parquet`]: bridges to the engine's sync parquet adapters via
//!   `spawn_blocking` (the parquet/arrow ecosystem is sync)

pub mod db;
pub mod host;
pub mod parquet;
