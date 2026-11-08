use crate::domain::indicator::{Indicator, IndicatorParameters};
use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::IndicatorValue;

const SECONDS_PER_DAY: i64 = 86_400;
pub const ONE_YEAR_SECS: i64 = 365 * SECONDS_PER_DAY + SECONDS_PER_DAY / 4;

// Operation Point fixtures

pub const fn op_points_empty() -> Vec<OperationPoint> {
    Vec::new()
}

fn winning_op(id: i32, timestamp: i64) -> OperationPoint {
    OperationPoint {
        instrument: "EURUSD".into(),
        result: 1,
        tp: 10,
        risk: 0.01,
        money_management_strategy_id: 1,
        id,
        sl: 5,
        timestamp,
    }
}

fn losing_op(id: i32, timestamp: i64) -> OperationPoint {
    OperationPoint {
        instrument: "EURUSD".into(),
        result: -1,
        tp: 8,
        risk: 0.02,
        money_management_strategy_id: 1,
        id,
        sl: 4,
        timestamp,
    }
}

pub fn op_points_default() -> Vec<OperationPoint> {
    vec![winning_op(1, 1000), losing_op(2, 1001)]
}

pub fn op_points_mms_id_2() -> Vec<OperationPoint> {
    op_points_default()
        .into_iter()
        .map(|p| OperationPoint {
            money_management_strategy_id: 2,
            ..p
        })
        .collect()
}

pub fn op_points_default_usdcad() -> Vec<OperationPoint> {
    op_points_default()
        .into_iter()
        .map(|p| OperationPoint {
            instrument: "USDCAD".into(),
            ..p
        })
        .collect()
}

pub fn op_points_partial_mismatch() -> Vec<OperationPoint> {
    vec![
        winning_op(1, 1000),
        OperationPoint {
            instrument: "USDCAD".into(),
            ..losing_op(2, 1001)
        },
    ]
}

pub fn op_points_mms_partial_mismatch() -> Vec<OperationPoint> {
    vec![
        winning_op(1, 1000),
        OperationPoint {
            money_management_strategy_id: 2,
            ..losing_op(2, 1001)
        },
    ]
}

pub fn op_points_ts_123() -> Vec<OperationPoint> {
    vec![winning_op(1, 1), losing_op(2, 2), losing_op(3, 3)]
}

pub fn op_points_ts_124() -> Vec<OperationPoint> {
    vec![winning_op(1, 1), losing_op(2, 2), losing_op(3, 4)]
}

pub fn op_point(id: i32, result: i32, timestamp: i64) -> OperationPoint {
    OperationPoint {
        instrument: "EURUSD".into(),
        result,
        tp: 30,
        sl: 20,
        risk: 0.02,
        money_management_strategy_id: 1,
        id,
        timestamp,
    }
}

// Resampled Point fixtures

pub const fn resampled_points_empty() -> Vec<ResampledPointD1> {
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
        },
    ]
}

// Indicator fixtures

pub const fn indicators_empty() -> Vec<Indicator> {
    Vec::new()
}

pub fn indicators_default() -> Vec<Indicator> {
    vec![
        Indicator {
            id: 1,
            parameters: IndicatorParameters::Macd {
                slow_type: "ema".to_string(),
                slow_n: 26,
                fast_type: "ema".to_string(),
                fast_n: 12,
            },
        },
        Indicator {
            id: 2,
            parameters: IndicatorParameters::Macd {
                slow_type: "ema".to_string(),
                slow_n: 26,
                fast_type: "ema".to_string(),
                fast_n: 12,
            },
        },
    ]
}

// Signal-generation fixtures: resampled-point series long enough to drive RSI
// windows, plus indicator builders with RSI parameters.

/// 5 resampled points with alternating close prices (1,2,1,2,1): yields 4
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
        },
    ]
}

/// A single RSI indicator (n=3, thresholds 70/30) with id 1.
pub fn indicators_rsi_single() -> Vec<Indicator> {
    vec![Indicator {
        id: 1,
        parameters: IndicatorParameters::Rsi {
            n: 3,
            high_threshold: 70,
            low_threshold: 30,
        },
    }]
}

// Signal fixtures

pub const fn iv(timestamp: i64, value: f64) -> IndicatorValue {
    IndicatorValue { timestamp, value }
}
