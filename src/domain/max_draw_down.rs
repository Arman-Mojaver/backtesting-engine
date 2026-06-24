use crate::domain::operation_point::OperationPoint;

pub fn max_draw_down(operation_points: &[OperationPoint]) -> f64 {
    if operation_points.is_empty() {
        return 0.0;
    }

    let mut cumsum: f64 = 1.0;
    let mut cummax: f64 = 1.0;
    let mut max_dd: f64 = 0.0;

    for point in operation_points {
        let contribution = point.result as f64 * point.risk / point.sl as f64;
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

    fn make_op(result: i32, risk: f64, sl: i32) -> OperationPoint {
        OperationPoint {
            instrument: "EURUSD".into(),
            result,
            tp: 10,
            risk,
            money_management_strategy_id: 1,
            id: 1,
            sl,
            timestamp: 1000,
        }
    }

    #[test]
    fn empty_ops_returns_zero() {
        assert_eq!(max_draw_down(&[]), 0.0);
    }

    #[test]
    fn single_winning_trade_returns_zero() {
        // contribution = 1 * 0.01 / 5 = 0.002
        // cumsum = 1.002, cummax = 1.002, dd = 0
        let ops = vec![make_op(1, 0.01, 5)];
        assert_eq!(max_draw_down(&ops), 0.0);
    }

    #[test]
    fn single_losing_trade_returns_drawdown() {
        // contribution = -1 * 0.02 / 4 = -0.005
        // cumsum = 0.995, cummax = 1.0, dd = (1.0 - 0.995) / 1.0 = 0.5%
        let ops = vec![make_op(-1, 0.02, 4)];
        assert_eq!(max_draw_down(&ops), 0.5);
    }

    #[test]
    fn peak_then_drawdown_captures_deepest_trough() {
        // trade 1: contribution = 1 * 0.01 / 5 = 0.002
        //          cumsum = 1.002, cummax = 1.002, dd = 0
        // trade 2: contribution = -5 * 0.01 / 5 = -0.01
        //          cumsum = 0.992, cummax = 1.002, dd = (1.002 - 0.992) / 1.002 ≈ 1.0%
        let ops = vec![make_op(1, 0.01, 5), make_op(-5, 0.01, 5)];
        assert!((max_draw_down(&ops) - 1.0).abs() < 0.1);
    }

    #[test]
    fn recovery_to_new_peak_keeps_earlier_max_dd() {
        // trade 1: contribution = -1 * 0.05 / 5 = -0.01
        //           cumsum = 0.99, cummax = 1.0, dd = (1.0 - 0.99) / 1.0 = 1.0%
        // trade 2: contribution = 3 * 0.05 / 5 = 0.03
        //           cumsum = 1.02, cummax = 1.02, dd = 0
        // max dd stays at 1.0%
        let ops = vec![make_op(-1, 0.05, 5), make_op(3, 0.05, 5)];
        assert!((max_draw_down(&ops) - 1.0).abs() < 0.1);
    }

    #[test]
    fn second_drawdown_deeper_than_first() {
        // trade 1: contribution = 1 * 0.5 / 5 = 0.1
        //           cumsum = 1.1, cummax = 1.1, dd = 0
        // trade 2: contribution = -1 * 0.5 / 5 = -0.1
        //           cumsum = 1.0, cummax = 1.1, dd = (1.1 - 1.0) / 1.1 ≈ 9.09%
        // trade 3: contribution = 1 * 0.5 / 5 = 0.1
        //           cumsum = 1.1, cummax = 1.1, dd = 0
        // trade 4: contribution = -2 * 0.5 / 5 = -0.2
        //           cumsum = 0.9, cummax = 1.1, dd = (1.1 - 0.9) / 1.1 ≈ 18.18% (deeper)
        let ops = vec![
            make_op(1, 0.5, 5),
            make_op(-1, 0.5, 5),
            make_op(1, 0.5, 5),
            make_op(-2, 0.5, 5),
        ];
        assert!((max_draw_down(&ops) - 18.18).abs() < 0.1);
    }

    #[test]
    fn higher_peak_then_deeper_drawdown() {
        // trade 1: contribution = 1 * 0.5 / 5 = 0.1
        //           cumsum = 1.1, cummax = 1.1, dd = 0
        // trade 2: contribution = -1 * 0.5 / 5 = -0.1
        //           cumsum = 1.0, cummax = 1.1, dd = (1.1 - 1.0) / 1.1 ≈ 9.09%
        // trade 3: contribution = 3 * 0.5 / 5 = 0.3
        //           cumsum = 1.3, cummax = 1.3, dd = 0 (higher peak)
        // trade 4: contribution = -4 * 0.5 / 5 = -0.4
        //           cumsum = 0.9, cummax = 1.3, dd = (1.3 - 0.9) / 1.3 ≈ 30.77%
        let ops = vec![
            make_op(1, 0.5, 5),
            make_op(-1, 0.5, 5),
            make_op(3, 0.5, 5),
            make_op(-4, 0.5, 5),
        ];
        assert!((max_draw_down(&ops) - 30.77).abs() < 0.1);
    }

    #[test]
    fn all_winners_returns_zero() {
        // contribution = 1 * 0.01 / 5 = 0.002 each
        // cumsum = 1.006, cummax = 1.006, dd = 0
        let ops = vec![
            make_op(1, 0.01, 5),
            make_op(1, 0.01, 5),
            make_op(1, 0.01, 5),
        ];
        assert_eq!(max_draw_down(&ops), 0.0);
    }

    #[test]
    fn all_losers_accumulate_drawdown() {
        // trade 1: contribution = -1 * 0.02 / 4 = -0.005
        //           cumsum = 0.995, cummax = 1.0, dd = (1.0 - 0.995) / 1.0 = 0.5%
        // trade 2: contribution = -1 * 0.02 / 4 = -0.005
        //           cumsum = 0.99, cummax = 1.0, dd = (1.0 - 0.99) / 1.0 = 1.0%
        let ops = vec![make_op(-1, 0.02, 4), make_op(-1, 0.02, 4)];
        assert!((max_draw_down(&ops) - 1.0).abs() < 0.1);
    }

    #[test]
    fn larger_losing_trade_after_winner() {
        // trade 1: contribution = 1 * 0.1 / 2 = 0.05
        //           cumsum = 1.05, cummax = 1.05, dd = 0
        // trade 2: contribution = -2 * 0.1 / 2 = -0.1
        //           cumsum = 0.95, cummax = 1.05, dd = (1.05 - 0.95) / 1.05 ≈ 9.52%
        let ops = vec![make_op(1, 0.1, 2), make_op(-2, 0.1, 2)];
        assert!((max_draw_down(&ops) - 9.52).abs() < 0.1);
    }

    #[test]
    fn oscillating_pattern_tracks_cumulative_trough() {
        // trade 1: contribution = 2 * 0.5 / 5 = 0.2
        //           cumsum = 1.2, cummax = 1.2, dd = 0
        // trade 2: contribution = -1 * 0.5 / 5 = -0.1
        //           cumsum = 1.1, cummax = 1.2, dd = (1.2 - 1.1) / 1.2 ≈ 8.33%
        // trade 3: contribution = 1 * 0.5 / 5 = 0.1
        //           cumsum = 1.2, cummax = 1.2, dd = 0
        // trade 4: contribution = -3 * 0.5 / 5 = -0.3
        //           cumsum = 0.9, cummax = 1.2, dd = (1.2 - 0.9) / 1.2 = 25.0%
        let ops = vec![
            make_op(2, 0.5, 5),
            make_op(-1, 0.5, 5),
            make_op(1, 0.5, 5),
            make_op(-3, 0.5, 5),
        ];
        assert_eq!(max_draw_down(&ops), 25.0);
    }

    #[test]
    fn rounding_to_4_decimal_places() {
        // contribution = -1 * 0.01 / 7 ≈ -0.001428
        // cumsum ≈ 0.99857, cummax = 1.0, dd ≈ 0.14%
        let ops = vec![make_op(-1, 0.01, 7)];
        assert_eq!(max_draw_down(&ops), 0.14);
    }

    #[test]
    fn rounding_up_at_boundary() {
        // contribution = -1 * 0.01 / 6 ≈ -0.001667
        // cumsum ≈ 0.99833, cummax = 1.0, dd ≈ 0.17%
        let ops = vec![make_op(-1, 0.01, 6)];
        assert_eq!(max_draw_down(&ops), 0.17);
    }

    #[test]
    fn sub_threshold_drawdown_rounds_to_zero() {
        // contribution = -1 * 0.0001 / 1000 = -0.0000001
        // cumsum ≈ 0.9999999, cummax = 1.0, dd = 0.00001% → rounds to 0.0
        let ops = vec![make_op(-1, 0.0001, 1000)];
        assert_eq!(max_draw_down(&ops), 0.0);
    }

    #[test]
    fn drawdown_exceeding_100_percent() {
        // trade 1: contribution = -1 * 0.6 / 1 = -0.6
        //           cumsum = 0.4, cummax = 1.0, dd = (1.0 - 0.4) / 1.0 = 60.0%
        // trade 2: contribution = -1 * 0.6 / 1 = -0.6
        //           cumsum = -0.2, cummax = 1.0, dd = (1.0 - (-0.2)) / 1.0 = 120.0%
        let ops = vec![make_op(-1, 0.6, 1), make_op(-1, 0.6, 1)];
        assert!((max_draw_down(&ops) - 120.0).abs() < 0.1);
    }
}
