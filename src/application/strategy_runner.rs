use std::collections::HashSet;

use anyhow::Context;
use log::debug;

use crate::application::signal_generator::{create_signal_groups, instrument_of};
use crate::application::strategy_generator::create_strategies;
use crate::domain::indicator::Indicator;
use crate::domain::operation_point::{OperationPoint, OperationPoints};
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal_group::filter_signal_groups_by_annual_ops;
use crate::domain::strategy::{filter_strategies, GlobalStrategyFilter, Strategy};
use crate::domain::validation::combined_validation::validate_all;

pub fn run_strategies(
    indicators: &[Indicator],
    resampled_points: &[ResampledPointD1],
    long_operation_points: &[OperationPoint],
    short_operation_points: &[OperationPoint],
    money_management_strategy_id: i32,
    unprocessed_ids: &[i32],
) -> anyhow::Result<Vec<Strategy>> {
    validate_all(
        long_operation_points,
        short_operation_points,
        indicators,
        resampled_points,
        money_management_strategy_id,
    )
    .context("Validation failed")?;

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
        global_filter.min_operation_count,
        start_timestamp,
        end_timestamp,
    );
    let instrument = instrument_of(resampled_points);
    debug!(
        "{instrument} (mms_id={money_management_strategy_id}): after annual operation count filter (min {}): {} signal groups",
        global_filter.min_operation_count,
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
    let unprocessed_ids: HashSet<i32> = unprocessed_ids.iter().copied().collect();
    indicators
        .iter()
        .filter(|indicator| unprocessed_ids.contains(&indicator.id))
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
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

    fn op(id: i32, result: i32, timestamp: i64) -> OperationPoint {
        OperationPoint {
            instrument: "EURUSD".to_string(),
            result,
            tp: 30,
            sl: 20,
            risk: 0.02,
            money_management_strategy_id: 1,
            id,
            timestamp,
        }
    }

    /// Long and short operation points share the bar-5 / bar-21 timestamps
    /// (validation requires identical timestamp sets). Short results are
    /// losers that no signal resolves: if one ever did, the 6.09 ROI
    /// assertions below would catch it.
    fn pipeline_long_ops() -> Vec<OperationPoint> {
        vec![
            op(1, 30, 4 * BAR_SPACING_SECS),
            op(2, 30, 20 * BAR_SPACING_SECS),
        ]
    }

    fn pipeline_short_ops() -> Vec<OperationPoint> {
        vec![
            op(10, -20, 4 * BAR_SPACING_SECS),
            op(11, -20, 20 * BAR_SPACING_SECS),
        ]
    }

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
            // 7: RSI(3) 60/40 — long signals at bars 5, 9, .., 25 and
            // short signals at bars 7, 11, .., 27 (12 signals/year). Only
            // the bar-5 and bar-21 longs resolve: two +30 trades compounding
            // 1.03 -> 1.0609 over exactly one year -> ROI 6.09, DD 0.
            rsi(7, 60, 40),
            // 8: RSI(3) 70/30 — values 33/67 never cross either threshold:
            // dropped at the activity stage before any strategy is built.
            rsi(8, 70, 30),
            // 9: ROC(5) — 11 signals (passes the 10/year activity bar) but
            // none land on an operation-point timestamp: ROI 0 fails the
            // quality bar -> dropped at the second stage.
            Indicator {
                id: 9,
                parameters: IndicatorParameters::Roc { period: 5 },
            },
            // 10: identical parameters to 7 — also passes both stages.
            rsi(10, 60, 40),
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

        // Stage 1 dropped indicator 8 (no signals); stage 2 dropped
        // indicator 9 (no resolved trades -> ROI 0 < 5); 7 and 10 survive.
        let ids: Vec<i32> = strategies.iter().map(|s| s.indicator_id).collect();
        assert_eq!(ids, vec![7, 10]);

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
        // Indicator 10 passes both filter stages in the full run above;
        // marking it processed must exclude it before any compute.
        let strategies = run_pipeline(1, &[7, 8, 9]).expect("valid inputs must pass validation");

        let ids: Vec<i32> = strategies.iter().map(|s| s.indicator_id).collect();
        assert_eq!(ids, vec![7]);
    }

    #[test]
    fn all_indicators_processed_yields_no_strategies() {
        // idempotent re-run: everything already processed
        let strategies = run_pipeline(1, &[]).expect("valid inputs must pass validation");

        assert!(strategies.is_empty());
    }

    #[test]
    fn validation_failure_surfaces_the_invariant_and_context() {
        // Every fixture operation point carries mms_id 1; expecting 2 must
        // fail validation with the context and the invariant named.
        let err =
            run_pipeline(2, &all_indicator_ids()).expect_err("mms mismatch must fail validation");
        let chain = format!("{err:#}");
        assert!(chain.contains("Validation failed"));
        assert!(chain.contains("money management strategy id"));
    }
}
