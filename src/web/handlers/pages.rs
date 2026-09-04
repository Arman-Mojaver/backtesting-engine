//! Page handlers: the full main page and the htmx stats fragment.

use std::sync::Arc;

use askama::Template;
use axum::extract::State;
use axum::response::Html;
use time::OffsetDateTime;

use crate::web::adapters::{db, host, parquet};
use crate::web::error::WebError;
use crate::web::info::{build_processing, DatabaseInfo, ProcessingInfo, SystemOverview};
use crate::web::state::AppState;

/// Renders the full main page.
pub async fn index(State(state): State<AppState>) -> Result<Html<String>, WebError> {
    let overview = collect_overview(&state).await;
    let template = IndexPage { data: &overview };
    Ok(Html(template.render()?))
}

/// Renders the auto-refreshing stats fragment (htmx swap target).
pub async fn stats_fragment(State(state): State<AppState>) -> Result<Html<String>, WebError> {
    let overview = collect_overview(&state).await;
    let template = StatsSections { data: &overview };
    Ok(Html(template.render()?))
}

/// Gathers the three sections concurrently, then assembles the view.
///
/// Host, DB, and parquet stats are collected via `tokio::join!` — the
/// sections are independent, so total latency is the slowest source, not
/// the sum. DB and parquet failures degrade gracefully into inline status
/// messages; only a template failure aborts the request.
async fn collect_overview(state: &AppState) -> SystemOverview {
    let (host_info, database, parquet_scan) = tokio::join!(
        host::collect(Arc::clone(&state.sys)),
        db::collect(&state.pool, &state.db_target),
        parquet::collect(state.parquet_root.clone()),
    );

    let (database, metadata) = match database {
        Ok(snapshot) => (snapshot.info, Some(snapshot.metadata)),
        Err(error) => (
            DatabaseInfo::unreachable(&state.db_target, &error.to_string()),
            None,
        ),
    };

    let processing = match parquet_scan {
        Ok(scan) => build_processing(metadata.as_ref(), scan),
        Err(error) => ProcessingInfo::unavailable(error.to_string()),
    };

    SystemOverview {
        environment: state.environment_label.clone(),
        generated_at: OffsetDateTime::now_utc().to_string(),
        host: host_info,
        database,
        processing,
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexPage<'a> {
    data: &'a SystemOverview,
}

#[derive(Template)]
#[template(path = "sections.html")]
struct StatsSections<'a> {
    data: &'a SystemOverview,
}
