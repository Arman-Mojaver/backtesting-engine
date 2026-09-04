//! Route table.

use axum::routing::get;
use axum::Router;

use crate::web::handlers::{assets, pages};
use crate::web::state::AppState;

/// Builds the application router.
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(pages::index))
        .route("/fragments/stats", get(pages::stats_fragment))
        .route("/static/style.css", get(assets::style_css))
        .route("/static/htmx.js", get(assets::htmx_js))
        .with_state(state)
}
