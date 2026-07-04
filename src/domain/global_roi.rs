use crate::domain::operation_point::OperationPoint;

pub fn global_roi(operation_points: &[&OperationPoint]) -> f64 {
    if operation_points.is_empty() {
        return 0.0;
    }

    let mut cumsum: f64 = 1.0;
    let mut cummax: f64 = 1.0;

    for point in operation_points {
        let contribution = cummax * point.result as f64 * (point.risk / point.sl as f64);
        cumsum += contribution;
        cummax = cummax.max(cumsum);
    }

    ((cumsum - 1.0) * 10000.0).round() / 100.0
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

    fn roi(ops: &[OperationPoint]) -> f64 {
        let refs: Vec<&OperationPoint> = ops.iter().collect();
        global_roi(&refs)
    }

    #[test]
    fn empty_ops_returns_zero() {
        assert_eq!(global_roi(&[]), 0.0);
    }

    #[test]
    fn single_negative_item() {
        // cumsum = 1 + 1 * (-20) * (0.02 / 20) = 1 - 0.02 = 0.98
        // roi = (0.98 - 1) * 100 = -2.0
        let ops = [make_op(-20, 30, 20, 0.02)];
        assert_eq!(roi(&ops), -2.0);
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
        assert_eq!(roi(&ops), -6.0);
    }

    #[test]
    fn single_positive_item() {
        // cumsum = 1 + 1 * 30 * (0.02 / 20) = 1 + 0.03 = 1.03
        // roi = (1.03 - 1) * 100 = 3.0
        let ops = [make_op(30, 30, 20, 0.02)];
        assert_eq!(roi(&ops), 3.0);
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
        assert_eq!(roi(&ops), 9.27);
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
        assert_eq!(roi(&ops), 4.03);
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
        assert_eq!(roi(&ops), 3.97);
    }

    #[test]
    fn rounding_to_two_decimal_places() {
        // cumsum = 1 + 1*1*(0.01/7) = 1 + 0.00142857... = 1.00142857
        // roi = 0.142857... -> 0.14
        let ops = [make_op(1, 10, 7, 0.01)];
        assert_eq!(roi(&ops), 0.14);
    }
}
