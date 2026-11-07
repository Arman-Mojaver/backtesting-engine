use anyhow::Context;
use log::info;

use crate::config::environment::DbConfig;
use crate::domain::indicator::{Indicator, IndicatorRepository};
use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};
use crate::infrastructure::pg_adapter::PgStore;

pub struct SourceData {
    pub indicators: Vec<Indicator>,
    pub resampled_points: Vec<ResampledPointD1>,
    pub long_operation_points: Vec<OperationPoint>,
    pub short_operation_points: Vec<OperationPoint>,
}

pub fn fetch_source_data(
    db_config: &DbConfig,
    instrument: &str,
    money_management_strategy_id: i32,
) -> anyhow::Result<SourceData> {
    info!("Connecting to {} ...", db_config.database);

    // One connection serves all four sequential reads; the batch path
    // (create_loop) opens one per table because its loads run in parallel.
    let store = PgStore::connect(db_config)?;

    let indicators = IndicatorRepository::get_all(&store).context("failed to fetch indicators")?;
    let resampled_points = ResampledPointD1Repository::get_by_instrument(&store, instrument)
        .with_context(|| format!("failed to fetch resampled points for {instrument}"))?;

    let long_operation_points: Vec<OperationPoint> =
        LongOperationPointRepository::get_by_mms_id_and_instrument(
            &store,
            money_management_strategy_id,
            instrument,
        )
        .with_context(|| {
            format!(
                "failed to fetch long operation points for {instrument} \
                 (mms_id={money_management_strategy_id})"
            )
        })?;

    let short_operation_points: Vec<OperationPoint> =
        ShortOperationPointRepository::get_by_mms_id_and_instrument(
            &store,
            money_management_strategy_id,
            instrument,
        )
        .with_context(|| {
            format!(
                "failed to fetch short operation points for {instrument} \
                 (mms_id={money_management_strategy_id})"
            )
        })?;

    Ok(SourceData {
        indicators,
        resampled_points,
        long_operation_points,
        short_operation_points,
    })
}
