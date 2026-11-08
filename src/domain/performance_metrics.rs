use crate::domain::cast;
use crate::domain::operation_point::OperationPoint;
use crate::domain::time_span::difference_in_years;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PerformanceMetrics {
    pub global_roi: f64,
    pub max_draw_down: f64,
}

pub fn performance_metrics(operation_points: &[&OperationPoint]) -> PerformanceMetrics {
    let mut sorted: Vec<&OperationPoint> = operation_points.to_vec();
    sorted.sort_by_key(|op| op.timestamp);

    let mut cumsum: f64 = 1.0;
    let mut cummax: f64 = 1.0;

    let mut max_dd: f64 = 0.0;

    for point in sorted {
        // The PG adapter rejects sl <= 0, so the division below stays finite.
        debug_assert!(
            point.sl > 0,
            "operation point {} has sl {}",
            point.id,
            point.sl
        );
        let contribution = cummax * f64::from(point.result) * (point.risk / f64::from(point.sl));
        cumsum += contribution;
        cummax = cummax.max(cumsum);

        let draw_down = (cummax - cumsum) / cummax;
        max_dd = max_dd.max(draw_down);
    }

    PerformanceMetrics {
        global_roi: cast::round_to((cumsum - 1.0) * 100.0, cast::METRIC_DECIMALS),
        max_draw_down: cast::round_to(max_dd * 100.0, cast::METRIC_DECIMALS),
    }
}

impl PerformanceMetrics {
    pub fn annual_roi(&self, start_timestamp: i64, end_timestamp: i64) -> f64 {
        let Some(years) = difference_in_years(start_timestamp, end_timestamp) else {
            return 0.0;
        };

        let global_accumulated = 1.0 + self.global_roi / 100.0;
        if global_accumulated <= 0.0 {
            return 0.0;
        }
        let annual_accumulated = global_accumulated.powf(1.0 / years);
        cast::round_to((annual_accumulated - 1.0) * 100.0, cast::METRIC_DECIMALS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- Shared fixtures -------------------------------------------------------

    fn make_op(result: i32, tp: i32, sl: i32, risk: f64) -> OperationPoint {
        make_op_at(result, tp, sl, risk, 1000)
    }

    // Like `make_op`, but with an explicit timestamp. The sorting tests need
    // distinct timestamps; every mapped test uses the historical default 1000.
    fn make_op_at(result: i32, tp: i32, sl: i32, risk: f64, timestamp: i64) -> OperationPoint {
        OperationPoint {
            instrument: "EURUSD".into(),
            result,
            tp,
            risk,
            money_management_strategy_id: 1,
            id: 1,
            sl,
            timestamp,
        }
    }

    fn metrics(ops: &[OperationPoint]) -> PerformanceMetrics {
        let refs: Vec<&OperationPoint> = ops.iter().collect();
        performance_metrics(&refs)
    }

    // Fixture for the `annual_roi` method: only `global_roi` is relevant there.
    fn metrics_with_roi(global_roi: f64) -> PerformanceMetrics {
        PerformanceMetrics {
            global_roi,
            max_draw_down: 0.0,
        }
    }

    // Maps to BOTH: global_roi::tests::empty_ops_returns_zero
    //               max_draw_down::tests::empty_ops_returns_zero
    #[test]
    fn empty_ops_returns_zero() {
        let m = performance_metrics(&[]);
        assert_eq!(m.global_roi, 0.0);
        assert_eq!(m.max_draw_down, 0.0);
    }

    #[test]
    fn metrics_on_valid_points_are_finite() {
        let fixtures: [&[OperationPoint]; 4] = [
            &[make_op(-20, 30, 20, 0.02)],
            &[make_op(30, 30, 20, 0.02), make_op(25, 25, 20, 0.02)],
            &[
                make_op(-20, 30, 20, 0.02),
                make_op(-20, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(25, 25, 20, 0.02),
                make_op(-200, 300, 200, 0.02),
                make_op(300, 300, 250, 0.02),
            ],
            &[
                make_op(-20, 30, 20, 0.02),
                make_op(-20, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(25, 25, 20, 0.02),
                make_op(-200, 300, 200, 0.02),
                make_op(-250, 300, 250, 0.02),
                make_op(-250, 300, 250, 0.02),
                make_op(25, 300, 250, 0.02),
                make_op(-250, 300, 250, 0.02),
            ],
        ];
        for ops in fixtures {
            let m = metrics(ops);
            assert!(m.global_roi.is_finite(), "global_roi {}", m.global_roi);
            assert!(
                m.max_draw_down.is_finite(),
                "max_draw_down {}",
                m.max_draw_down
            );
        }
    }

    // =========================================================================
    // Mapped 1:1 from src/domain/global_roi.rs::tests; names, inputs and
    // expectations preserved verbatim; each now asserts the `global_roi`
    // field of the combined result.
    // =========================================================================
    mod global_roi {
        use super::*;

        #[test]
        fn single_negative_item() {
            // cumsum = 1 + 1 * (-20) * (0.02 / 20) = 1 - 0.02 = 0.98
            // roi = (0.98 - 1) * 100 = -2.0
            let ops = [make_op(-20, 30, 20, 0.02)];
            assert_eq!(metrics(&ops).global_roi, -2.0);
        }

        #[test]
        fn several_negative_items() {
            // cumsum = 1 + (-20)*(0.02/20) + (-40)*(0.02/40) + (-50)*(0.02/50)
            //        = 1 - 0.02 - 0.02 - 0.02 = 0.94
            // roi = (0.94 - 1) * 100 = -6.0
            let ops = [
                make_op(-20, 30, 20, 0.02),
                make_op(-40, 30, 40, 0.02),
                make_op(-50, 30, 50, 0.02),
            ];
            assert_eq!(metrics(&ops).global_roi, -6.0);
        }

        #[test]
        fn single_positive_item() {
            // cumsum = 1 + 1 * 30 * (0.02 / 20) = 1 + 0.03 = 1.03
            // roi = (1.03 - 1) * 100 = 3.0
            let ops = [make_op(30, 30, 20, 0.02)];
            assert_eq!(metrics(&ops).global_roi, 3.0);
        }

        #[test]
        fn several_positive_items_compound_against_cummax() {
            // trade 1: cumsum = 1 + 1*30*(0.02/20) = 1.03, cummax = 1.03
            // trade 2: cumsum = 1.03 + 1.03*30*(0.02/20) = 1.03 + 0.0309 = 1.0609, cummax = 1.0609
            // trade 3: cumsum = 1.0609 + 1.0609*30*(0.02/20) = 1.0609 + 0.031827 = 1.092727
            // roi = (1.092727 - 1) * 100 = 9.27 (rounded 2dp)
            let ops = [
                make_op(30, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
            ];
            assert_eq!(metrics(&ops).global_roi, 9.27);
        }

        #[test]
        fn negative_then_positive_items() {
            // trade 1: cumsum = 1 + 1*(-20)*(0.02/20) = 0.98, cummax = 1.0
            // trade 2: cumsum = 0.98 + 1.0*30*(0.02/20) = 0.98 + 0.03 = 1.01, cummax = 1.01
            // trade 3: cumsum = 1.01 + 1.01*30*(0.02/20) = 1.01 + 0.0303 = 1.0403
            // roi = (1.0403 - 1) * 100 = 4.03
            let ops = [
                make_op(-20, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
            ];
            assert_eq!(metrics(&ops).global_roi, 4.03);
        }

        #[test]
        fn positive_then_negative_items() {
            // trade 1: cumsum = 1 + 1*30*(0.02/20) = 1.03, cummax = 1.03
            // trade 2: cumsum = 1.03 + 1.03*30*(0.02/20) = 1.0609, cummax = 1.0609
            // trade 3: cumsum = 1.0609 + 1.0609*(-20)*(0.02/20) = 1.0609 - 0.021218 = 1.039682
            // roi = (1.039682 - 1) * 100 = 3.97
            let ops = [
                make_op(30, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(-20, 30, 20, 0.02),
            ];
            assert_eq!(metrics(&ops).global_roi, 3.97);
        }

        #[test]
        fn rounding_to_two_decimal_places() {
            // cumsum = 1 + 1*1*(0.01/7) = 1 + 0.00142857... = 1.00142857
            // roi = 0.142857... -> 0.14
            let ops = [make_op(1, 10, 7, 0.01)];
            assert_eq!(metrics(&ops).global_roi, 0.14);
        }
    }

    // =========================================================================
    // Mapped 1:1 from src/domain/max_draw_down.rs::tests; names, inputs and
    // expectations preserved verbatim; each now asserts the `max_draw_down`
    // field of the combined result.
    // =========================================================================
    mod max_draw_down {
        use super::*;

        #[test]
        fn only_positive_results_returns_zero_1() {
            let ops = [make_op(20, 20, 20, 0.02)];
            assert_eq!(metrics(&ops).max_draw_down, 0.0);
        }

        #[test]
        fn only_positive_results_returns_zero_2() {
            let ops = [
                make_op(20, 20, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(40, 40, 20, 0.02),
            ];
            assert_eq!(metrics(&ops).max_draw_down, 0.0);
        }

        #[test]
        fn one_negative_result() {
            // cumsum = 1 + 1*(-20)*(0.02/20) = 0.98; dd = (1-0.98)/1 = 2.0%
            let ops = [make_op(-20, 30, 20, 0.02)];
            assert_eq!(metrics(&ops).max_draw_down, 2.0);
        }

        #[test]
        fn three_negative_results() {
            // each contribution = -0.02 (cummax stays 1); cumsum = 0.94; dd = 6.0%
            let ops = [
                make_op(-20, 30, 20, 0.02),
                make_op(-35, 30, 35, 0.02),
                make_op(-50, 30, 50, 0.02),
            ];
            assert_eq!(metrics(&ops).max_draw_down, 6.0);
        }

        #[test]
        fn positive_then_negative() {
            // t1: cumsum=1.03, cummax=1.03
            // t2: 1.03*(-0.02)=-0.0206; cumsum=1.0094; dd=(1.03-1.0094)/1.03=2.0%
            let ops = [make_op(30, 30, 20, 0.02), make_op(-20, 30, 20, 0.02)];
            assert_eq!(metrics(&ops).max_draw_down, 2.0);
        }

        #[test]
        fn negative_then_positive() {
            // t1: cumsum=0.98, cummax=1; dd=2.0%
            // t2: 1*0.03=0.03; cumsum=1.01; dd=0
            // max dd = 2.0%
            let ops = [make_op(-20, 30, 20, 0.02), make_op(30, 30, 20, 0.02)];
            assert_eq!(metrics(&ops).max_draw_down, 2.0);
        }

        #[test]
        fn positive_then_two_negatives() {
            // t1: cumsum=1.03, cummax=1.03
            // t2: 1.03*(-0.02)=-0.0206; cumsum=1.0094; dd=2.0%
            // t3: 1.03*(-0.02)=-0.0206; cumsum=0.9888; dd=(1.03-0.9888)/1.03=4.0%
            let ops = [
                make_op(30, 30, 20, 0.02),
                make_op(-20, 30, 20, 0.02),
                make_op(-20, 30, 20, 0.02),
            ];
            assert_eq!(metrics(&ops).max_draw_down, 4.0);
        }

        #[test]
        fn two_negatives_then_three_positives() {
            // t1: cumsum=0.98, dd=2.0%
            // t2: cumsum=0.96, dd=4.0%
            // t3: 1*0.03; cumsum=0.99; dd=1.0%
            // t4: 1*0.025; cumsum=1.015; cummax=1.015; dd=0
            // t5: 1.015*0.03; cumsum=1.04545; cummax=1.04545; dd=0
            // max dd = 4.0%
            let ops = [
                make_op(-20, 30, 20, 0.02),
                make_op(-20, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
            ];
            assert_eq!(metrics(&ops).max_draw_down, 4.0);
        }

        #[test]
        fn mixed_order_case_1() {
            let ops = [
                make_op(-20, 30, 20, 0.02),
                make_op(-20, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(25, 25, 20, 0.02),
                make_op(-200, 300, 200, 0.02),
                make_op(300, 300, 250, 0.02),
            ];
            assert_eq!(metrics(&ops).max_draw_down, 4.0);
        }

        #[test]
        fn mixed_order_case_2() {
            let ops = [
                make_op(-20, 30, 20, 0.02),
                make_op(-20, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(25, 25, 20, 0.02),
                make_op(-200, 300, 200, 0.02),
                make_op(-250, 300, 250, 0.02),
                make_op(-250, 300, 250, 0.02),
            ];
            assert_eq!(metrics(&ops).max_draw_down, 6.0);
        }

        #[test]
        fn mixed_order_case_with_decimals() {
            let ops = [
                make_op(-20, 30, 20, 0.02),
                make_op(-20, 30, 20, 0.02),
                make_op(30, 30, 20, 0.02),
                make_op(25, 25, 20, 0.02),
                make_op(-200, 300, 200, 0.02),
                make_op(-250, 300, 250, 0.02),
                make_op(-250, 300, 250, 0.02),
                make_op(25, 300, 250, 0.02),
                make_op(-250, 300, 250, 0.02),
            ];
            assert_eq!(metrics(&ops).max_draw_down, 7.8);
        }
    }

    // =========================================================================
    // NEW tests, no 1:1 ancestor. Cover the defensive sort inside
    // `performance_metrics()`: unsorted input must be computed in
    // chronological order, and equal timestamps must keep input order.
    // =========================================================================
    mod sorting {
        use super::*;

        #[test]
        fn unsorted_input_computed_in_chronological_order() {
            // Same trades as max_draw_down::positive_then_two_negatives
            // (expected: roi -1.12, dd 4.0) but with distinct timestamps,
            // passed in scrambled order. Chronological order is
            // [ts=1000: +30, ts=2000: -20, ts=3000: -20]:
            //   t1: cumsum = 1.03, cummax = 1.03
            //   t2: cumsum = 1.0094, dd = 2.0%
            //   t3: cumsum = 0.9888, dd = 4.0%
            //   roi = (0.9888 - 1) * 100 = -1.12
            // If the function ignored ordering and used the given order
            // [-20, +30, -20], it would instead return roi -1.02 / dd 2.0.
            let ops = [
                make_op_at(-20, 30, 20, 0.02, 3000), // last chronologically
                make_op_at(30, 30, 20, 0.02, 1000),  // first chronologically
                make_op_at(-20, 30, 20, 0.02, 2000), // second chronologically
            ];
            let m = metrics(&ops);
            assert_eq!(m.global_roi, -1.12);
            assert_eq!(m.max_draw_down, 4.0);
        }

        #[test]
        fn equal_timestamps_preserve_input_order() {
            // Both ops share a timestamp, so the stable sort must keep the
            // given order [-20, +30]:
            //   t1: cumsum = 0.98, cummax = 1
            //   t2: cumsum = 1.01
            //   roi = 1.0
            // A flipped order [+30, -20] would give cumsum 1.03 then 1.0094,
            // i.e. roi 0.94, so this asserts the tie-break, not just the math.
            let ops = [make_op(-20, 30, 20, 0.02), make_op(30, 30, 20, 0.02)];
            assert_eq!(metrics(&ops).global_roi, 1.0);
        }
    }

    // =========================================================================
    // Mapped 1:1 from src/domain/annual_roi.rs::tests; names, inputs and
    // expectations preserved verbatim. The free function
    // `annual_roi(start, end, roi)` became a method:
    // `metrics_with_roi(roi).annual_roi(start, end)`.
    // =========================================================================
    mod annual_roi {
        use super::*;

        use crate::domain::fixtures::ONE_YEAR_SECS;

        const HALF_YEAR_SECS: i64 = ONE_YEAR_SECS / 2;
        const QUARTER_YEAR_SECS: i64 = ONE_YEAR_SECS / 4;
        const THIRD_YEAR_SECS: i64 = ONE_YEAR_SECS / 3;

        #[test]
        fn zero_span_returns_zero() {
            assert_eq!(metrics_with_roi(10.0).annual_roi(1000, 1000), 0.0);
        }

        #[test]
        fn negative_span_returns_zero() {
            assert_eq!(metrics_with_roi(10.0).annual_roi(2000, 1000), 0.0);
        }

        #[test]
        fn one_year_compounds_to_same_global_roi() {
            // 1 year: annual = global
            assert_eq!(metrics_with_roi(10.0).annual_roi(0, ONE_YEAR_SECS), 10.0);
        }

        #[test]
        fn two_years_compounds_to_global() {
            // 2 years, global 21%: annual = ((1.21)^(1/2) - 1) * 100 = 10.0
            assert_eq!(
                metrics_with_roi(21.0).annual_roi(0, 2 * ONE_YEAR_SECS),
                10.0
            );
        }

        #[test]
        fn three_years_compounds_to_global() {
            // 3 years, global 33.1%: annual = ((1.331)^(1/3) - 1) * 100 = 10.0
            assert_eq!(
                metrics_with_roi(33.1).annual_roi(0, 3 * ONE_YEAR_SECS),
                10.0
            );
        }

        #[test]
        fn four_years_compounds_to_global() {
            // 4 years, global 46.41%: annual = ((1.4641)^(1/4) - 1) * 100 = 10.0
            assert_eq!(
                metrics_with_roi(46.41).annual_roi(0, 4 * ONE_YEAR_SECS),
                10.0
            );
        }

        #[test]
        fn five_years_compounds_to_global() {
            // 5 years, global 61.05%: annual = ((1.6105)^(1/5) - 1) * 100 = 10.0
            assert_eq!(
                metrics_with_roi(61.05).annual_roi(0, 5 * ONE_YEAR_SECS),
                10.0
            );
        }

        #[test]
        fn half_year_compounds_to_global() {
            // 0.5 years, global 4.88%: annual = ((1.0488)^(1/0.5) - 1) * 100 = 10.0
            assert_eq!(metrics_with_roi(4.88).annual_roi(0, HALF_YEAR_SECS), 10.0);
        }

        #[test]
        fn one_and_half_years_compounds_to_global() {
            // 1.5 years, global 15.37%: annual = ((1.1537)^(1/1.5) - 1) * 100 = 10.0
            assert_eq!(
                metrics_with_roi(15.37).annual_roi(0, ONE_YEAR_SECS + HALF_YEAR_SECS),
                10.0
            );
        }

        #[test]
        fn quarter_year_compounds_to_global() {
            // 0.25 years, global 2.411%: annual = ((1.02411)^(1/0.25) - 1) * 100 = 10.0
            assert_eq!(
                metrics_with_roi(2.411).annual_roi(0, QUARTER_YEAR_SECS),
                10.0
            );
        }

        #[test]
        fn third_of_year_compounds_to_global() {
            // 0.33 years, global 3.228%: annual = ((1.03228)^(1/0.33) - 1) * 100 = 10.0
            assert_eq!(metrics_with_roi(3.228).annual_roi(0, THIRD_YEAR_SECS), 10.0);
        }

        #[test]
        fn offset_start_produces_same_result_as_zero_start() {
            let from_zero = metrics_with_roi(10.0).annual_roi(0, ONE_YEAR_SECS);
            let from_offset =
                metrics_with_roi(10.0).annual_roi(1_000_000, 1_000_000 + ONE_YEAR_SECS);
            assert_eq!(from_zero, from_offset);
        }

        #[test]
        fn negative_global_roi_annualises() {
            // 1 year, global -10%: annual = -10.0
            assert_eq!(metrics_with_roi(-10.0).annual_roi(0, ONE_YEAR_SECS), -10.0);
        }

        #[test]
        fn zero_global_roi_returns_zero() {
            assert_eq!(metrics_with_roi(0.0).annual_roi(0, ONE_YEAR_SECS), 0.0);
        }

        #[test]
        fn total_loss_global_roi_returns_zero_not_nan() {
            // global_roi = -100 -> accumulated = 0.0: nothing left to
            // annualize; the guard returns 0.0.
            assert_eq!(metrics_with_roi(-100.0).annual_roi(0, ONE_YEAR_SECS), 0.0);
        }

        #[test]
        fn beyond_total_loss_global_roi_returns_zero_not_nan() {
            // global_roi = -150 -> accumulated = -0.5: powf on a negative
            // base would be NaN; the guard returns 0.0 instead.
            assert_eq!(metrics_with_roi(-150.0).annual_roi(0, ONE_YEAR_SECS), 0.0);
        }

        #[test]
        fn just_above_total_loss_still_annualizes() {
            // global_roi = -99 -> accumulated = 0.01 -> annual = -99.0.
            assert_eq!(metrics_with_roi(-99.0).annual_roi(0, ONE_YEAR_SECS), -99.0);
        }
    }
}
