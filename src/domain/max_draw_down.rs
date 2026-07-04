use crate::domain::operation_point::OperationPoint;

pub fn max_draw_down(operation_points: &[&OperationPoint]) -> f64 {
    if operation_points.is_empty() {
        return 0.0;
    }

    let mut cumsum: f64 = 1.0;
    let mut cummax: f64 = 1.0;
    let mut max_dd: f64 = 0.0;

    for point in operation_points {
        let contribution = cummax * point.result as f64 * (point.risk / point.sl as f64);
        cumsum += contribution;
        cummax = cummax.max(cumsum);
        let draw_down = (cummax - cumsum) / cummax;
        max_dd = max_dd.max(draw_down);
    }

    (max_dd * 10000.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_op(result: i32, tp: i32, sl: i32, risk: f64) -> OperationPoint {
        OperationPoint {
            instrument: "EURUSD".into(),
            result,
            tp,
            risk,
            money_management_strategy_id: 1,
            id: 1,
            sl,
            timestamp: 1000,
        }
    }

    fn dd(ops: &[OperationPoint]) -> f64 {
        let refs: Vec<&OperationPoint> = ops.iter().collect();
        max_draw_down(&refs)
    }

    #[test]
    fn empty_ops_returns_zero() {
        assert_eq!(max_draw_down(&[]), 0.0);
    }

    #[test]
    fn only_positive_results_returns_zero_1() {
        let ops = [make_op(20, 20, 20, 0.02)];
        assert_eq!(dd(&ops), 0.0);
    }

    #[test]
    fn only_positive_results_returns_zero_2() {
        let ops = [
            make_op(20, 20, 20, 0.02),
            make_op(30, 30, 20, 0.02),
            make_op(40, 40, 20, 0.02),
        ];
        assert_eq!(dd(&ops), 0.0);
    }

    #[test]
    fn one_negative_result() {
        // cumsum = 1 + 1*(-20)*(0.02/20) = 0.98; dd = (1-0.98)/1 = 2.0%
        let ops = [make_op(-20, 30, 20, 0.02)];
        assert_eq!(dd(&ops), 2.0);
    }

    #[test]
    fn three_negative_results() {
        // each contribution = -0.02 (cummax stays 1); cumsum = 0.94; dd = 6.0%
        let ops = [
            make_op(-20, 30, 20, 0.02),
            make_op(-35, 30, 35, 0.02),
            make_op(-50, 30, 50, 0.02),
        ];
        assert_eq!(dd(&ops), 6.0);
    }

    #[test]
    fn positive_then_negative() {
        // t1: cumsum=1.03, cummax=1.03
        // t2: 1.03*(-0.02)=-0.0206; cumsum=1.0094; dd=(1.03-1.0094)/1.03=2.0%
        let ops = [make_op(30, 30, 20, 0.02), make_op(-20, 30, 20, 0.02)];
        assert_eq!(dd(&ops), 2.0);
    }

    #[test]
    fn negative_then_positive() {
        // t1: cumsum=0.98, cummax=1; dd=2.0%
        // t2: 1*0.03=0.03; cumsum=1.01; dd=0
        // max dd = 2.0%
        let ops = [make_op(-20, 30, 20, 0.02), make_op(30, 30, 20, 0.02)];
        assert_eq!(dd(&ops), 2.0);
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
        assert_eq!(dd(&ops), 4.0);
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
        assert_eq!(dd(&ops), 4.0);
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
        assert_eq!(dd(&ops), 4.0);
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
        assert_eq!(dd(&ops), 6.0);
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
        assert_eq!(dd(&ops), 7.8);
    }
}
