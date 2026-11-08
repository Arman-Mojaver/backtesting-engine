use anyhow::Context;
use log::debug;
use rustc_hash::FxHashSet;

use crate::application::signal_generator::{create_signal_groups, instrument_of};
use crate::application::strategy_generator::create_strategies;
use crate::domain::indicator::Indicator;
use crate::domain::operation_point::{OperationPoint, OperationPoints};
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal_group::filter_signal_groups_by_annual_ops;
use crate::domain::strategy::{filter_strategies, GlobalStrategyFilter, Strategy};
use crate::domain::validation::validate_all;

pub fn run_strategies(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    long_operation_points: &[OperationPoint],
    short_operation_points: &[OperationPoint],
    money_management_strategy_id: i32,
    unprocessed_ids: &[i32],
) -> anyhow::Result<Vec<Strategy>> {
    validate_all(
        indicators,
        resampled_points,
        long_operation_points,
        short_operation_points,
        money_management_strategy_id,
    )
    .context("input validation failed")?;

    let operation_points = OperationPoints {
        long_operation_points,
        short_operation_points,
    };
    let (start_timestamp, end_timestamp) = operation_points
        .start_end_timestamps()
        .ok_or_else(|| anyhow::anyhow!("long operation points should never be empty"))?;

    let unprocessed_indicators = unprocessed_indicators(indicators, unprocessed_ids);
    let signal_groups = create_signal_groups(&unprocessed_indicators, resampled_points);

    let global_filter = GlobalStrategyFilter::default();
    let filtered_groups = filter_signal_groups_by_annual_ops(
        signal_groups,
        global_filter.min_annual_operation_count,
        start_timestamp,
        end_timestamp,
    );
    let instrument = instrument_of(resampled_points);
    debug!(
        "{instrument} (mms_id={money_management_strategy_id}): after annual operation count filter (min {}): {} signal groups",
        global_filter.min_annual_operation_count,
        filtered_groups.len(),
    );

    let strategies = create_strategies(
        &filtered_groups,
        &operation_points,
        money_management_strategy_id,
        start_timestamp,
        end_timestamp,
    );

    let strategies = filter_strategies(strategies, &global_filter);
    debug!(
        "{instrument} (mms_id={money_management_strategy_id}): after strategy filter (min annual ROI {}, max drawdown {}): {} strategies",
        global_filter.min_annual_roi,
        global_filter.max_draw_down_max,
        strategies.len(),
    );

    Ok(strategies)
}

fn unprocessed_indicators(indicators: &[Indicator], unprocessed_ids: &[i32]) -> Vec<Indicator> {
    let unprocessed_ids: FxHashSet<i32> = unprocessed_ids.iter().copied().collect();
    indicators
        .iter()
        .filter(|indicator| unprocessed_ids.contains(&indicator.id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::op_point;
    use crate::domain::indicator::IndicatorParameters;

    /// Bar spacing chosen so bars 5 and 21 are exactly one year apart:
    /// 16 * BAR_SPACING_SECS = 31_557_600 (365.25 days). With the long
    /// operation points sitting on bars 5 and 21, every annualized figure
    /// in these tests is computed over exactly one year.
    const BAR_SPACING_SECS: i64 = 1_972_350;

    /// Closes repeat [5, 4, 3, 4]: diffs repeat [-1, -1, +1, +1]. RSI(3)
    /// windows then always hold both gains and losses, so the series is
    /// 33.33 / 66.67 / 66.67 / 33.33 by bar index (mod 4) — deterministic
    /// threshold crossings with no zero-gain/zero-loss edge case.
    fn pipeline_resampled_points() -> Vec<ResampledPointD1> {
        let closes = [5.0, 4.0, 3.0, 4.0];
        (0..28)
            .map(|k: i32| {
                let close = closes[usize::try_from(k % 4).unwrap()];
                ResampledPointD1 {
                    id: k.saturating_add(1),
                    instrument: "EURUSD".to_string(),
                    open: close,
                    high: close + 0.1,
                    low: close - 0.1,
                    close,
                    volume: 1000,
                    timestamp: i64::from(k).saturating_mul(BAR_SPACING_SECS),
                }
            })
            .collect()
    }

    /// Long and short operation points share the bar-5 / bar-21 timestamps
    /// (validation requires identical timestamp sets). Short results are
    /// losers that no signal resolves: if one ever did, the 6.09 ROI
    /// assertions below would catch it.
    fn pipeline_long_ops() -> Vec<OperationPoint> {
        vec![
            op_point(1, 30, 4 * BAR_SPACING_SECS),
            op_point(2, 30, 20 * BAR_SPACING_SECS),
        ]
    }

    fn pipeline_short_ops() -> Vec<OperationPoint> {
        vec![
            op_point(10, -20, 4 * BAR_SPACING_SECS),
            op_point(11, -20, 20 * BAR_SPACING_SECS),
        ]
    }

    const PASSES_BOTH_STAGES: i32 = 7;
    const DROPPED_FOR_INACTIVITY: i32 = 8;
    const DROPPED_FOR_LOW_ROI: i32 = 9;
    const DUPLICATE_OF_PASSES_BOTH_STAGES: i32 = 10;

    fn rsi(id: i32, high: i32, low: i32) -> Indicator {
        Indicator {
            id,
            parameters: IndicatorParameters::Rsi {
                n: 3,
                high_threshold: high,
                low_threshold: low,
            },
        }
    }

    fn pipeline_indicators() -> Vec<Indicator> {
        vec![
            rsi(PASSES_BOTH_STAGES, 60, 40),
            rsi(DROPPED_FOR_INACTIVITY, 70, 30),
            Indicator {
                id: DROPPED_FOR_LOW_ROI,
                parameters: IndicatorParameters::Roc { period: 5 },
            },
            rsi(DUPLICATE_OF_PASSES_BOTH_STAGES, 60, 40),
        ]
    }

    fn all_indicator_ids() -> Vec<i32> {
        pipeline_indicators().iter().map(|i| i.id).collect()
    }

    fn run_pipeline(
        money_management_strategy_id: i32,
        unprocessed_ids: &[i32],
    ) -> anyhow::Result<Vec<Strategy>> {
        run_strategies(
            &pipeline_indicators(),
            &pipeline_resampled_points(),
            &pipeline_long_ops(),
            &pipeline_short_ops(),
            money_management_strategy_id,
            unprocessed_ids,
        )
    }

    #[test]
    fn full_pipeline_applies_validation_and_both_filter_stages() {
        let strategies =
            run_pipeline(1, &all_indicator_ids()).expect("valid inputs must pass validation");

        let ids: Vec<i32> = strategies.iter().map(|s| s.indicator_id).collect();
        assert_eq!(
            ids,
            vec![PASSES_BOTH_STAGES, DUPLICATE_OF_PASSES_BOTH_STAGES]
        );

        let s = &strategies[0];
        assert_eq!(s.instrument, "EURUSD");
        assert_eq!(s.money_management_strategy_id, 1);
        // Two +30 trades compounding at risk/sl = 0.001: 1.03 -> 1.0609
        // over exactly one year (bars 5 to 21), so annual == global.
        assert_eq!(s.annual_roi, 6.09);
        assert_eq!(s.max_draw_down, 0.0);
        assert_eq!(s.annual_operation_count, 2.0);
        assert_eq!(s.long_operation_point_ids, vec![1, 2]);
        assert!(s.short_operation_point_ids.is_empty());
    }

    #[test]
    fn processed_indicators_are_excluded_before_compute() {
        let unprocessed_ids = [
            PASSES_BOTH_STAGES,
            DROPPED_FOR_INACTIVITY,
            DROPPED_FOR_LOW_ROI,
        ];
        let strategies =
            run_pipeline(1, &unprocessed_ids).expect("valid inputs must pass validation");

        let ids: Vec<i32> = strategies.iter().map(|s| s.indicator_id).collect();
        assert_eq!(ids, vec![PASSES_BOTH_STAGES]);
    }

    #[test]
    fn all_indicators_processed_yields_no_strategies() {
        let strategies = run_pipeline(1, &[]).expect("valid inputs must pass validation");

        assert!(strategies.is_empty());
    }

    #[test]
    fn validation_failure_surfaces_the_invariant_and_context() {
        let err =
            run_pipeline(2, &all_indicator_ids()).expect_err("mms mismatch must fail validation");
        let chain = format!("{err:#}");
        assert!(chain.contains("input validation failed"));
        assert!(chain.contains("money management strategy id"));
    }
}
