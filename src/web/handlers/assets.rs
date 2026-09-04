//! Embedded static assets — compiled into the binary, no filesystem serving.

use axum::http::header;
use axum::response::IntoResponse;

const STYLE_CSS: &str = include_str!("../static/style.css");
const HTMX_JS: &str = include_str!("../static/htmx.min.js");

/// Serves the embedded stylesheet.
pub async fn style_css() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/css; charset=utf-8")],
        STYLE_CSS,
    )
}

/// Serves the vendored htmx script (htmx 2.0.10, https://htmx.org).
pub async fn htmx_js() -> impl IntoResponse {
    (
        [(header::CONTENT_TYPE, "text/javascript; charset=utf-8")],
        HTMX_JS,
    )
}
