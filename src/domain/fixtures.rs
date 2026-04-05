use crate::domain::indicator::Indicator;
use crate::domain::operation_point::OperationPoint;
use crate::domain::resampled_point::ResampledPointD1;

pub fn op_points_empty() -> Vec<OperationPoint> {
    Vec::new()
}

pub fn resampled_points_empty() -> Vec<ResampledPointD1> {
    Vec::new()
}

pub fn indicators_empty() -> Vec<Indicator> {
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

pub fn indicators_default() -> Vec<Indicator> {
    vec![
        Indicator { id: 1 },
        Indicator { id: 2 },
    ]
}

/// Default resampled points
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