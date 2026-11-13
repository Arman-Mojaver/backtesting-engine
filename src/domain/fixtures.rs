use crate::domain::indicator::{Indicator, IndicatorParameters, MovingAverageType};
use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;
use crate::domain::signal::IndicatorValue;

const SECONDS_PER_DAY: i64 = 86_400;
pub const ONE_YEAR_SECS: i64 = 365 * SECONDS_PER_DAY + SECONDS_PER_DAY / 4;

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
    vec![winning_op(1, 1000), losing_op(2, 2000), losing_op(3, 3000)]
}

pub fn op_points_ts_124() -> Vec<OperationPoint> {
    vec![winning_op(1, 1000), losing_op(2, 2000), losing_op(3, 4000)]
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

fn base_resampled_point(id: i32, timestamp: i64) -> ResampledPointD1 {
    ResampledPointD1 {
        id,
        instrument: "EURUSD".into(),
        open: 1.0,
        high: 1.1,
        low: 0.9,
        close: 1.05,
        volume: 1000,
        timestamp,
    }
}

fn later_resampled_point(id: i32, timestamp: i64) -> ResampledPointD1 {
    ResampledPointD1 {
        id,
        instrument: "EURUSD".into(),
        open: 1.2,
        high: 1.25,
        low: 1.15,
        close: 1.22,
        volume: 800,
        timestamp,
    }
}

pub fn resampled_points_at(timestamps: &[i64]) -> Vec<ResampledPointD1> {
    (1..)
        .zip(timestamps)
        .map(|(id, &timestamp)| base_resampled_point(id, timestamp))
        .collect()
}

pub fn resampled_points_default() -> Vec<ResampledPointD1> {
    vec![
        base_resampled_point(1, 1000),
        later_resampled_point(2, 1001),
    ]
}

pub fn resampled_points_ts_123() -> Vec<ResampledPointD1> {
    vec![
        base_resampled_point(1, 1000),
        later_resampled_point(2, 2000),
        later_resampled_point(3, 3000),
    ]
}

pub fn resampled_points_usdcad() -> Vec<ResampledPointD1> {
    vec![
        ResampledPointD1 {
            instrument: "USDCAD".into(),
            ..base_resampled_point(1, 1000)
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
        base_resampled_point(1, 1000),
        ResampledPointD1 {
            instrument: "USDCAD".into(),
            ..later_resampled_point(2, 1001)
        },
    ]
}

fn ema_macd(id: i32) -> Indicator {
    Indicator {
        id,
        parameters: IndicatorParameters::Macd {
            slow_type: MovingAverageType::Ema,
            slow_n: 26,
            fast_type: MovingAverageType::Ema,
            fast_n: 12,
        },
    }
}

pub fn indicators_default() -> Vec<Indicator> {
    vec![ema_macd(1), ema_macd(2)]
}

const ALTERNATING_CLOSES: [f64; 5] = [1.0, 2.0, 1.0, 2.0, 1.0];

/// Four close-to-close diffs: enough for RSI windows up to n = 4.
pub fn resampled_points_for_rsi() -> Vec<ResampledPointD1> {
    (1..)
        .zip(ALTERNATING_CLOSES)
        .map(|(id, close)| ResampledPointD1 {
            id,
            instrument: "EURUSD".into(),
            open: close,
            high: close + 0.1,
            low: close - 0.1,
            close,
            volume: 1000,
            timestamp: i64::from(id).saturating_mul(1000),
        })
        .collect()
}

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

pub const fn iv(timestamp: i64, value: f64) -> IndicatorValue {
    IndicatorValue { timestamp, value }
}
