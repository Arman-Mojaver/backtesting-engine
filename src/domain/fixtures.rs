use crate::domain::indicator::{Indicator, IndicatorParameters};
use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1};

// Operation Point fixtures

pub fn op_points_empty() -> Vec<OperationPoint> {
    Vec::new()
}
pub fn op_points_default() -> Vec<OperationPoint> {
    vec![
        OperationPoint {
            instrument: "EURUSD".into(),
            result: 1,
            tp: 10,
            risk: 0.01,
            money_management_strategy_id: 1,
            id: 1,
            sl: 5,
            timestamp: 1000,
        },
        OperationPoint {
            instrument: "EURUSD".into(),
            result: -1,
            tp: 8,
            risk: 0.02,
            money_management_strategy_id: 1,
            id: 2,
            sl: 4,
            timestamp: 1001,
        },
    ]
}

pub fn op_points_mms_id_2() -> Vec<OperationPoint> {
    vec![
        OperationPoint {
            instrument: "EURUSD".into(),
            result: 1,
            tp: 10,
            risk: 0.01,
            money_management_strategy_id: 2,
            id: 1,
            sl: 5,
            timestamp: 1000,
        },
        OperationPoint {
            instrument: "EURUSD".into(),
            result: -1,
            tp: 8,
            risk: 0.02,
            money_management_strategy_id: 2,
            id: 2,
            sl: 4,
            timestamp: 1001,
        },
    ]
}

pub fn op_points_default_usdcad() -> Vec<OperationPoint> {
    vec![
        OperationPoint {
            instrument: "USDCAD".into(),
            result: 1,
            tp: 10,
            risk: 0.01,
            money_management_strategy_id: 1,
            id: 1,
            sl: 5,
            timestamp: 1000,
        },
        OperationPoint {
            instrument: "USDCAD".into(),
            result: -1,
            tp: 8,
            risk: 0.02,
            money_management_strategy_id: 1,
            id: 2,
            sl: 4,
            timestamp: 1001,
        },
    ]
}

pub fn op_points_partial_mismatch() -> Vec<OperationPoint> {
    vec![
        OperationPoint {
            instrument: "EURUSD".into(),
            result: 1,
            tp: 10,
            risk: 0.01,
            money_management_strategy_id: 1,
            id: 1,
            sl: 5,
            timestamp: 1000,
        },
        OperationPoint {
            instrument: "USDCAD".into(),
            result: -1,
            tp: 8,
            risk: 0.02,
            money_management_strategy_id: 1,
            id: 2,
            sl: 4,
            timestamp: 1001,
        },
    ]
}

pub fn op_points_mms_partial_mismatch() -> Vec<OperationPoint> {
    vec![
        OperationPoint {
            instrument: "EURUSD".into(),
            result: 1,
            tp: 10,
            risk: 0.01,
            money_management_strategy_id: 1,
            id: 1,
            sl: 5,
            timestamp: 1000,
        },
        OperationPoint {
            instrument: "EURUSD".into(),
            result: -1,
            tp: 8,
            risk: 0.02,
            money_management_strategy_id: 2,
            id: 2,
            sl: 4,
            timestamp: 1001,
        },
    ]
}

pub fn op_points_ts_123() -> Vec<OperationPoint> {
    vec![
        OperationPoint {
            instrument: "EURUSD".into(),
            result: 1,
            tp: 10,
            risk: 0.01,
            money_management_strategy_id: 1,
            id: 1,
            sl: 5,
            timestamp: 1,
        },
        OperationPoint {
            instrument: "EURUSD".into(),
            result: -1,
            tp: 8,
            risk: 0.02,
            money_management_strategy_id: 1,
            id: 2,
            sl: 4,
            timestamp: 2,
        },
        OperationPoint {
            instrument: "EURUSD".into(),
            result: -1,
            tp: 8,
            risk: 0.02,
            money_management_strategy_id: 1,
            id: 3,
            sl: 4,
            timestamp: 3,
        },
    ]
}

pub fn op_points_ts_124() -> Vec<OperationPoint> {
    vec![
        OperationPoint {
            instrument: "EURUSD".into(),
            result: 1,
            tp: 10,
            risk: 0.01,
            money_management_strategy_id: 1,
            id: 1,
            sl: 5,
            timestamp: 1,
        },
        OperationPoint {
            instrument: "EURUSD".into(),
            result: -1,
            tp: 8,
            risk: 0.02,
            money_management_strategy_id: 1,
            id: 2,
            sl: 4,
            timestamp: 2,
        },
        OperationPoint {
            instrument: "EURUSD".into(),
            result: -1,
            tp: 8,
            risk: 0.02,
            money_management_strategy_id: 1,
            id: 3,
            sl: 4,
            timestamp: 4,
        },
    ]
}

// Resampled Point fixtures

pub fn resampled_points_empty() -> Vec<ResampledPointD1> {
    Vec::new()
}
pub fn resampled_points_default() -> Vec<ResampledPointD1> {
    vec![
        ResampledPointD1 {
            id: 1,
            instrument: "EURUSD".into(),
            open: 1.0,
            high: 1.1,
            low: 0.9,
            close: 1.05,
            volume: 1000,
            timestamp: 1000,
            high_low_order: HighLowOrder::HighFirst,
        },
        ResampledPointD1 {
            id: 2,
            instrument: "EURUSD".into(),
            open: 1.2,
            high: 1.25,
            low: 1.15,
            close: 1.22,
            volume: 800,
            timestamp: 1001,
            high_low_order: HighLowOrder::HighFirst,
        },
    ]
}

pub fn resampled_points_ts_123() -> Vec<ResampledPointD1> {
    vec![
        ResampledPointD1 {
            id: 1,
            instrument: "EURUSD".into(),
            open: 1.0,
            high: 1.1,
            low: 0.9,
            close: 1.05,
            volume: 1000,
            timestamp: 1,
            high_low_order: HighLowOrder::HighFirst,
        },
        ResampledPointD1 {
            id: 2,
            instrument: "EURUSD".into(),
            open: 1.2,
            high: 1.25,
            low: 1.15,
            close: 1.22,
            volume: 800,
            timestamp: 2,
            high_low_order: HighLowOrder::HighFirst,
        },
        ResampledPointD1 {
            id: 3,
            instrument: "EURUSD".into(),
            open: 1.2,
            high: 1.25,
            low: 1.15,
            close: 1.22,
            volume: 800,
            timestamp: 3,
            high_low_order: HighLowOrder::HighFirst,
        },
    ]
}

pub fn resampled_points_usdcad() -> Vec<ResampledPointD1> {
    vec![
        ResampledPointD1 {
            id: 1,
            instrument: "USDCAD".into(),
            open: 1.0,
            high: 1.1,
            low: 0.9,
            close: 1.05,
            volume: 1000,
            timestamp: 1000,
            high_low_order: HighLowOrder::HighFirst,
        },
        ResampledPointD1 {
            id: 2,
            instrument: "USDCAD".into(),
            open: 1.05,
            high: 1.15,
            low: 0.95,
            close: 1.10,
            volume: 900,
            timestamp: 1001,
            high_low_order: HighLowOrder::HighFirst,
        },
    ]
}
pub fn resampled_points_partial_mismatch() -> Vec<ResampledPointD1> {
    vec![
        ResampledPointD1 {
            id: 1,
            instrument: "EURUSD".into(),
            open: 1.0,
            high: 1.1,
            low: 0.9,
            close: 1.05,
            volume: 1000,
            timestamp: 1000,
            high_low_order: HighLowOrder::HighFirst,
        },
        ResampledPointD1 {
            id: 2,
            instrument: "USDCAD".into(),
            open: 1.2,
            high: 1.25,
            low: 1.15,
            close: 1.22,
            volume: 800,
            timestamp: 1001,
            high_low_order: HighLowOrder::HighFirst,
        },
    ]
}

// Indicator fixtures

pub fn indicators_empty() -> Vec<Indicator> {
    Vec::new()
}

pub fn indicators_default() -> Vec<Indicator> {
    vec![
        Indicator {
            id: 1,
            r#type: "macd".to_string(),
            parameters: IndicatorParameters::Macd {
                slow_type: "ema".to_string(),
                slow_n: 26,
                fast_type: "ema".to_string(),
                fast_n: 12,
            },
            identifier: "macd_1".to_string(),
        },
        Indicator {
            id: 2,
            r#type: "macd".to_string(),
            parameters: IndicatorParameters::Macd {
                slow_type: "ema".to_string(),
                slow_n: 26,
                fast_type: "ema".to_string(),
                fast_n: 12,
            },
            identifier: "macd_2".to_string(),
        },
    ]
}

pub fn indicators_different_type() -> Vec<Indicator> {
    vec![
        Indicator {
            id: 1,
            r#type: "macd".to_string(),
            parameters: IndicatorParameters::Macd {
                slow_type: "ema".to_string(),
                slow_n: 26,
                fast_type: "ema".to_string(),
                fast_n: 12,
            },
            identifier: "macd_1".to_string(),
        },
        Indicator {
            id: 2,
            r#type: "rsi".to_string(),
            parameters: IndicatorParameters::Rsi {
                n: 14,
                high_threshold: 70,
                low_threshold: 30,
            },
            identifier: "rsi_1".to_string(),
        },
    ]
}

// Signal-generation fixtures: resampled-point series long enough to drive RSI
// windows, plus indicator builders with RSI parameters.

/// 5 resampled points with alternating close prices (1,2,1,2,1) — yields 4
/// price diffs and supports RSI windows of n up to 4. Enough to exercise the
/// RSI computation path through `create_signals`.
pub fn resampled_points_for_rsi() -> Vec<ResampledPointD1> {
    vec![
        ResampledPointD1 {
            id: 1,
            instrument: "EURUSD".into(),
            open: 1.0,
            high: 1.1,
            low: 0.9,
            close: 1.0,
            volume: 1000,
            timestamp: 1000,
            high_low_order: HighLowOrder::HighFirst,
        },
        ResampledPointD1 {
            id: 2,
            instrument: "EURUSD".into(),
            open: 2.0,
            high: 2.1,
            low: 1.9,
            close: 2.0,
            volume: 1000,
            timestamp: 2000,
            high_low_order: HighLowOrder::HighFirst,
        },
        ResampledPointD1 {
            id: 3,
            instrument: "EURUSD".into(),
            open: 1.0,
            high: 1.1,
            low: 0.9,
            close: 1.0,
            volume: 1000,
            timestamp: 3000,
            high_low_order: HighLowOrder::HighFirst,
        },
        ResampledPointD1 {
            id: 4,
            instrument: "EURUSD".into(),
            open: 2.0,
            high: 2.1,
            low: 1.9,
            close: 2.0,
            volume: 1000,
            timestamp: 4000,
            high_low_order: HighLowOrder::HighFirst,
        },
        ResampledPointD1 {
            id: 5,
            instrument: "EURUSD".into(),
            open: 1.0,
            high: 1.1,
            low: 0.9,
            close: 1.0,
            volume: 1000,
            timestamp: 5000,
            high_low_order: HighLowOrder::HighFirst,
        },
    ]
}

/// A single RSI indicator (n=3, thresholds 70/30) with id 1.
pub fn indicators_rsi_single() -> Vec<Indicator> {
    vec![Indicator {
        id: 1,
        r#type: "rsi".to_string(),
        parameters: IndicatorParameters::Rsi {
            n: 3,
            high_threshold: 70,
            low_threshold: 30,
        },
        identifier: "rsi_3-70-30".to_string(),
    }]
}
