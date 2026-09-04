//! System-info web service.
//!
//! Async axum application rendering server-side HTML (askama) with htmx for
//! live refresh. All async code lives in this module. It reuses the sync
//! `config`/`application`/`infrastructure` layers as-is: parquet adapters run
//! behind `spawn_blocking` bridges (the parquet ecosystem is sync), while the
//! DB is queried through this module's own async adapter (tokio-postgres
//! pool). Nothing outside `src/web` knows about async or this service.

pub mod adapters;
pub mod error;
pub mod handlers;
pub mod info;
pub mod routes;
pub mod state;

use anyhow::{Context, Result};
use log::info;

/// Default listen port (override with the `PORT` environment variable).
const DEFAULT_PORT: u16 = 3000;

/// Builds the application state, wires the router, and serves until shutdown.
pub async fn run() -> Result<()> {
    let environment = state::resolve_environment();
    let state = state::build_state(environment).await?;
    let app = routes::router(state);

    let port = std::env::var("PORT")
        .ok()
        .and_then(|raw| raw.parse().ok())
        .unwrap_or(DEFAULT_PORT);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .with_context(|| format!("failed to bind 0.0.0.0:{port}"))?;

    info!("webapp listening on 0.0.0.0:{port}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;
    Ok(())
}

/// Resolves when the process receives SIGINT — the graceful-shutdown signal.
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("shutdown signal received");
}
