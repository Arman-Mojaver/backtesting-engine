//! Webapp entrypoint (internal dashboard).
//!
//! Thin wrapper: initializes logging, then hands off to [`web::run`].

use backtesting_engine::infrastructure::logging;
use backtesting_engine::web;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    logging::init()?;
    web::run().await
}
