//! Typed web errors and their HTTP mapping.
//!
//! Every data source degrades gracefully (a DB or parquet failure is shown
//! inline on the page), so only template rendering can abort a request.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

/// Errors that abort a request.
#[derive(Debug, thiserror::Error)]
pub enum WebError {
    #[error("template render error: {0}")]
    Template(#[from] askama::Error),
}

impl IntoResponse for WebError {
    fn into_response(self) -> Response {
        // Internal tool: surface the error text with a 500.
        (StatusCode::INTERNAL_SERVER_ERROR, self.to_string()).into_response()
    }
}
