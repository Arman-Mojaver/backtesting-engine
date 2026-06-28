use crate::application::strategy_data::{fetch_strategy_data, StrategyQuery};
use crate::application::strategy_generator::generate_random_strategies;
use crate::config::environment::parquet_base_dir;
use crate::domain::operation_point::OperationPoints;
use crate::domain::validation::combined_validation::{validate_all, CombinedValidationError};
use crate::infrastructure::strategy_adapter::ParquetStrategyRepository;

use super::resolve_environment;

pub fn execute(money_management_strategy_id: i32, instrument: String) -> anyhow::Result<()> {
    let query = StrategyQuery {
        money_management_strategy_id,
        instrument: instrument.clone(),
    };

    eprintln!("Fetching data from database ...");
    let data = fetch_strategy_data(&query)?;

    eprintln!("Validating data ...");
    validate_all(
        &data.long_operation_points,
        &data.short_operation_points,
        &data.indicators,
        &data.resampled_points,
        money_management_strategy_id,
    )
    .map_err(|e: CombinedValidationError| anyhow::anyhow!("Validation failed: {:?}", e))?;

    let operation_points = OperationPoints{
        long_operation_points: data.long_operation_points,
        short_operation_points:data.short_operation_points,
    };

    let (start_timestamp, end_timestamp) = operation_points.start_end_timestamps();
    eprintln!("start_timestamp {}:, end_timestamp: {}.", start_timestamp, end_timestamp);

    eprintln!("Generating random strategies ...");
    let strategies = generate_random_strategies(
        &data.indicators,
        &operation_points.long_operation_points,
        &operation_points.short_operation_points,
        money_management_strategy_id,
    );

    let env = resolve_environment();
    let base_dir = parquet_base_dir(env);
    let repo = ParquetStrategyRepository::new(base_dir.clone());

    eprintln!("Writing {} strategies to parquet ...", strategies.len());
    repo.write(&instrument, money_management_strategy_id, &strategies)?;

    eprintln!(
        "Done. Wrote {} strategies for {} (mms_id={})",
        strategies.len(),
        instrument,
        money_management_strategy_id,
    );
    Ok(())
}
