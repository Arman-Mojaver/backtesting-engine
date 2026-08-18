use anyhow::Context;

use crate::config::environment::{DbConfig, Environment};
use crate::domain::indicator::{Indicator, IndicatorRepository};
use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};
use crate::infrastructure::pg_adapter::{
    PgIndicatorAdapter, PgLongOperationPointAdapter, PgResampledPointAdapter,
    PgShortOperationPointAdapter,
};

pub struct SourceData {
    pub money_management_strategy_id: i32,
    pub indicators: Vec<Indicator>,
    pub resampled_points: Vec<ResampledPointD1>,
    pub long_operation_points: Vec<OperationPoint>,
    pub short_operation_points: Vec<OperationPoint>,
}

pub fn fetch_source_data(
    instrument: &str,
    money_management_strategy_id: i32,
) -> anyhow::Result<SourceData> {
    let env: Environment = std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .parse()
        .unwrap_or(Environment::Development);
    let config = DbConfig::from_env(env);
    let conn_string = config.connection_string();

    eprintln!("Connecting to {} ...", config.database);

    let indicator_adapter =
        PgIndicatorAdapter::new(&conn_string).with_context(|| connection_diagnostics(&config))?;
    let long_adapter = PgLongOperationPointAdapter::new(&conn_string)
        .with_context(|| connection_diagnostics(&config))?;
    let short_adapter = PgShortOperationPointAdapter::new(&conn_string)
        .with_context(|| connection_diagnostics(&config))?;
    let resampled_adapter = PgResampledPointAdapter::new(&conn_string)
        .with_context(|| connection_diagnostics(&config))?;

    let indicators = indicator_adapter.get_all();
    let resampled_points = resampled_adapter.get_by_instrument(instrument);

    let long_operation_points: Vec<OperationPoint> =
        long_adapter.get_by_mms_id_and_instrument(money_management_strategy_id, instrument);

    let short_operation_points: Vec<OperationPoint> =
        short_adapter.get_by_mms_id_and_instrument(money_management_strategy_id, instrument);

    Ok(SourceData {
        money_management_strategy_id,
        indicators,
        resampled_points,
        long_operation_points,
        short_operation_points,
    })
}

fn connection_diagnostics(config: &DbConfig) -> String {
    format!(
        "Failed to connect to PostgreSQL\n  \
         Target database: {db}\n  \
         Target host:     {host}:{port}\n  \
         Target user:     {user}\n  \
         Check that:
  - The database container is running (`docker ps`)
  - The port is reachable from the engine container (try `psql -h {host} -p {port} -U {user} -d {db}`)
  - The role \"{user}\" exists in the target database
  - No local service is occupying port {port} on the host",
        db = config.database,
        host = config.host,
        port = config.port,
        user = config.user,
    )
}
