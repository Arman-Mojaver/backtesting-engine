use crate::application::signal_generator::create_signal_groups;
use crate::application::strategy_data::{fetch_strategy_data, StrategyQuery};
use crate::application::strategy_generator::create_strategies;
use crate::config::environment::parquet_base_dir;
use crate::domain::operation_point::OperationPoints;
use crate::domain::signal_group::filter_signal_groups_by_annual_ops;
use crate::domain::strategy::{filter_strategies, GlobalStrategyFilter};
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

    let operation_points = OperationPoints {
        long_operation_points: data.long_operation_points,
        short_operation_points: data.short_operation_points,
    };

    let (start_timestamp, end_timestamp) = operation_points.start_end_timestamps();
    eprintln!(
        "start_timestamp {}:, end_timestamp: {}.",
        start_timestamp, end_timestamp
    );

    eprintln!("Creating signals ...");
    let signal_groups = create_signal_groups(&data.indicators, &data.resampled_points);
    eprintln!(
        "Signal Groups {} signal groups (total operations: {})",
        signal_groups.len(),
        signal_groups
            .iter()
            .map(|sg| sg.operation_count)
            .sum::<usize>()
    );

    let global_strategy_filter = GlobalStrategyFilter::default();
    let signal_groups = filter_signal_groups_by_annual_ops(
        signal_groups,
        global_strategy_filter.min_operation_count,
        start_timestamp,
        end_timestamp,
    );
    eprintln!(
        "After annual operation count filter (min {}): {} signal groups",
        global_strategy_filter.min_operation_count,
        signal_groups.len()
    );

    eprintln!("Creating strategies ...");
    let strategies = create_strategies(
        &signal_groups,
        &operation_points,
        money_management_strategy_id,
        start_timestamp,
        end_timestamp,
    );

    let strategies = filter_strategies(strategies, &global_strategy_filter);
    eprintln!(
        "After strategy filter (min annual ROI {}, max drawdown {}): {} strategies",
        global_strategy_filter.min_annual_roi,
        global_strategy_filter.max_draw_down_max,
        strategies.len()
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
