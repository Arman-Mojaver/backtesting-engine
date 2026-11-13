use anyhow::Context;
use log::debug;

use crate::application::signal_generator::{create_signal_groups, instrument_of};
use crate::application::strategy_generator::create_strategies;
use crate::domain::indicator::Indicator;
use crate::domain::operation_point::{OperationPoint, OperationPoints};
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal_group::filter_signal_groups_by_annual_ops;
use crate::domain::strategy::{GlobalStrategyFilter, Strategy};
use crate::domain::validation::validate_all;

pub fn run_strategies(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    long_operation_points: &[OperationPoint],
    short_operation_points: &[OperationPoint],
    money_management_strategy_id: i32,
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

    let signal_groups = create_signal_groups(indicators, resampled_points);

    let global_filter = GlobalStrategyFilter::default();
    let filtered_groups = filter_signal_groups_by_annual_ops(
        signal_groups,
        &global_filter,
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
        &global_filter,
        money_management_strategy_id,
        start_timestamp,
        end_timestamp,
    );
    debug!(
        "{instrument} (mms_id={money_management_strategy_id}): after strategy filter (min annual ROI {}, max drawdown {}): {} strategies",
        global_filter.min_annual_roi,
        global_filter.max_draw_down_max,
        strategies.len(),
    );

    Ok(strategies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::fixtures::op_point;
    use crate::domain::indicator::IndicatorParameters;

    /// Bar spacing chosen so bars 6 and 26 are exactly one year apart:
    /// 20 * BAR_SPACING_SECS = 31_557_600 (365.25 days). With the operation
    /// points spanning bars 6 to 26, every annualized figure in these tests
    /// is computed over exactly one year.
    const BAR_SPACING_SECS: i64 = 1_577_880;

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

    const OPERATION_BARS: std::ops::RangeInclusive<i32> = 6..=26;
    const SHORT_ID_OFFSET: i32 = 100;

    /// Long and short operation points sit on every bar from 6 to 26
    /// (validation requires identical timestamp sets), so each RSI(3)
    /// indicator matches 10 trades in the year. The passing RSI crosses on
    /// bars 0 mod 4 (long) and 2 mod 4 (short) and enters on the next bar,
    /// so winners sit only on bars 1 mod 4 (longs) and 3 mod 4 (shorts).
    /// Every other operation point loses.
    fn pipeline_long_ops() -> Vec<OperationPoint> {
        pipeline_ops(1, 0)
    }

    fn pipeline_short_ops() -> Vec<OperationPoint> {
        pipeline_ops(3, SHORT_ID_OFFSET)
    }

    fn pipeline_ops(winning_bar_mod_4: i32, id_offset: i32) -> Vec<OperationPoint> {
        OPERATION_BARS
            .map(|bar| {
                let result = if bar % 4 == winning_bar_mod_4 {
                    30
                } else {
                    -20
                };
                op_point(
                    id_offset.saturating_add(bar),
                    result,
                    i64::from(bar).saturating_mul(BAR_SPACING_SECS),
                )
            })
            .collect()
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

    fn run_pipeline(money_management_strategy_id: i32) -> anyhow::Result<Vec<Strategy>> {
        run_strategies(
            &pipeline_indicators(),
            &pipeline_resampled_points(),
            &pipeline_long_ops(),
            &pipeline_short_ops(),
            money_management_strategy_id,
        )
    }

    #[test]
    fn full_pipeline_applies_validation_and_both_filter_stages() {
        let strategies = run_pipeline(1).expect("valid inputs must pass validation");

        let ids: Vec<i32> = strategies.iter().map(|s| s.indicator_id).collect();
        assert_eq!(
            ids,
            vec![PASSES_BOTH_STAGES, DUPLICATE_OF_PASSES_BOTH_STAGES]
        );

        let s = &strategies[0];
        assert_eq!(s.instrument, "EURUSD");
        assert_eq!(s.money_management_strategy_id, 1);
        // Ten +30 trades compounding at risk/sl = 0.001: 1.03^10 = 1.3439
        // over exactly one year (bars 6 to 26), so annual == global.
        assert_eq!(s.annual_roi, 34.39);
        assert_eq!(s.max_draw_down, 0.0);
        assert_eq!(s.annual_operation_count, 10.0);
        assert_eq!(s.long_operation_point_ids, vec![9, 13, 17, 21, 25]);
        assert_eq!(s.short_operation_point_ids, vec![107, 111, 115, 119, 123]);
    }

    #[test]
    fn validation_failure_surfaces_the_invariant_and_context() {
        let err = run_pipeline(2).expect_err("mms mismatch must fail validation");
        let chain = format!("{err:#}");
        assert!(chain.contains("input validation failed"));
        assert!(chain.contains("money management strategy id"));
    }

    #[test]
    fn reversed_resampled_points_stop_the_pipeline_at_validation() {
        let mut resampled_points = pipeline_resampled_points();
        resampled_points.reverse();

        let err = run_strategies(
            &pipeline_indicators(),
            &resampled_points,
            &pipeline_long_ops(),
            &pipeline_short_ops(),
            1,
        )
        .expect_err("reversed resampled points must fail validation");

        let chain = format!("{err:#}");
        assert!(chain.contains("input validation failed"));
        assert!(chain.contains("resampled points must be in strictly increasing timestamp order"));
    }
}
