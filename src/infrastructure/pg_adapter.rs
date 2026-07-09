use std::cell::RefCell;

use postgres::{Client, NoTls, Row};

use crate::domain::indicator::{Indicator, IndicatorParameters, IndicatorRepository};
use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1, ResampledPointD1Repository};

fn json_extract_str(json: &str, key: &str) -> String {
    let search = format!("\"{key}\":\"");
    let start = json
        .find(&search)
        .unwrap_or_else(|| panic!("key \"{key}\" not found in JSON: {json}"));
    let value_start = start + search.len();
    let value_end = json[value_start..]
        .find('"')
        .unwrap_or_else(|| panic!("unterminated string for key \"{key}\" in JSON: {json}"));
    json[value_start..value_start + value_end].to_string()
}

fn json_extract_i32(json: &str, key: &str) -> i32 {
    let search = format!("\"{key}\":");
    let start = json
        .find(&search)
        .unwrap_or_else(|| panic!("key \"{key}\" not found in JSON: {json}"));
    let value_start = start + search.len();
    let value_slice = &json[value_start..];
    let value_slice = value_slice.trim_start();
    let end = value_slice
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(value_slice.len());
    value_slice[..end].parse().unwrap_or_else(|_| {
        panic!(
            "failed to parse i32 for key \"{key}\" from: {}",
            &value_slice[..end]
        )
    })
}

fn map_high_low_order(value: &str) -> HighLowOrder {
    match value {
        "high_first" => HighLowOrder::HighFirst,
        "low_first" => HighLowOrder::LowFirst,
        _ => HighLowOrder::Undefined,
    }
}

fn map_parameters(indicator_type: &str, json_text: &str) -> IndicatorParameters {
    match indicator_type {
        "macd" => IndicatorParameters::Macd {
            slow_type: json_extract_str(json_text, "slow_type"),
            slow_n: json_extract_i32(json_text, "slow_n"),
            fast_type: json_extract_str(json_text, "fast_type"),
            fast_n: json_extract_i32(json_text, "fast_n"),
        },
        "rsi" => IndicatorParameters::Rsi {
            n: json_extract_i32(json_text, "n"),
            high_threshold: json_extract_i32(json_text, "high_threshold"),
            low_threshold: json_extract_i32(json_text, "low_threshold"),
        },
        "awesome_oscillator" => IndicatorParameters::AwesomeOscillator,
        "accelerator_decelerator" => IndicatorParameters::AcceleratorDecelerator,
        _ => panic!("unknown indicator type: {indicator_type}"),
    }
}

fn row_to_operation_point(row: &Row) -> OperationPoint {
    OperationPoint {
        id: row.get("id"),
        instrument: row.get("instrument"),
        result: row.get("result"),
        tp: row.get("tp"),
        sl: row.get("sl"),
        risk: row.get("risk"),
        timestamp: row.get::<_, i32>("timestamp") as i64,
        money_management_strategy_id: row.get("money_management_strategy_id"),
    }
}

fn row_to_resampled_point(row: &Row) -> ResampledPointD1 {
    ResampledPointD1 {
        id: row.get("id"),
        instrument: row.get("instrument"),
        open: row.get("open"),
        high: row.get("high"),
        low: row.get("low"),
        close: row.get("close"),
        volume: row.get("volume"),
        timestamp: row.get::<_, i32>("timestamp") as i64,
        high_low_order: map_high_low_order(&row.get::<_, String>("high_low_order")),
    }
}

fn row_to_indicator(row: &Row) -> Indicator {
    let raw_type: String = row.get("type");
    let raw_params: String = row.get("parameters");
    Indicator {
        id: row.get("id"),
        r#type: raw_type.clone(),
        parameters: map_parameters(&raw_type, &raw_params),
        identifier: row.get("identifier"),
    }
}

fn connect(connection_string: &str) -> anyhow::Result<Client> {
    Client::connect(connection_string, NoTls).map_err(anyhow::Error::from)
}

pub struct PgLongOperationPointAdapter {
    conn: RefCell<Client>,
}

impl PgLongOperationPointAdapter {
    pub fn new(connection_string: &str) -> anyhow::Result<Self> {
        let conn = connect(connection_string)?;
        Ok(Self {
            conn: RefCell::new(conn),
        })
    }
}

impl LongOperationPointRepository for PgLongOperationPointAdapter {
    fn get_all(&self) -> Vec<OperationPoint> {
        let rows = self
            .conn
            .borrow_mut()
            .query(
                "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
                 FROM long_operation_point",
                &[],
            )
            .expect("failed to query long_operation_point");
        rows.iter().map(row_to_operation_point).collect()
    }

    fn get_by_mms_id_and_instrument(&self, mms_id: i32, instrument: &str) -> Vec<OperationPoint> {
        let rows = self
            .conn
            .borrow_mut()
            .query(
                "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
                 FROM long_operation_point \
                 WHERE money_management_strategy_id = $1 AND instrument = $2",
                &[&mms_id, &instrument],
            )
            .expect("failed to query long_operation_point by mms_id and instrument");
        rows.iter().map(row_to_operation_point).collect()
    }
}

pub struct PgShortOperationPointAdapter {
    conn: RefCell<Client>,
}

impl PgShortOperationPointAdapter {
    pub fn new(connection_string: &str) -> anyhow::Result<Self> {
        let conn = connect(connection_string)?;
        Ok(Self {
            conn: RefCell::new(conn),
        })
    }
}

impl ShortOperationPointRepository for PgShortOperationPointAdapter {
    fn get_all(&self) -> Vec<OperationPoint> {
        let rows = self
            .conn
            .borrow_mut()
            .query(
                "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
                 FROM short_operation_point",
                &[],
            )
            .expect("failed to query short_operation_point");
        rows.iter().map(row_to_operation_point).collect()
    }

    fn get_by_mms_id_and_instrument(&self, mms_id: i32, instrument: &str) -> Vec<OperationPoint> {
        let rows = self
            .conn
            .borrow_mut()
            .query(
                "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
                 FROM short_operation_point \
                 WHERE money_management_strategy_id = $1 AND instrument = $2",
                &[&mms_id, &instrument],
            )
            .expect("failed to query short_operation_point by mms_id and instrument");
        rows.iter().map(row_to_operation_point).collect()
    }
}

pub struct PgResampledPointAdapter {
    conn: RefCell<Client>,
}

impl PgResampledPointAdapter {
    pub fn new(connection_string: &str) -> anyhow::Result<Self> {
        let conn = connect(connection_string)?;
        Ok(Self {
            conn: RefCell::new(conn),
        })
    }
}

impl ResampledPointD1Repository for PgResampledPointAdapter {
    fn get_all(&self) -> Vec<ResampledPointD1> {
        let rows = self
            .conn
            .borrow_mut()
            .query(
                "SELECT id, instrument, open, high, low, close, volume, timestamp, high_low_order \
                 FROM resampled_point_d1",
                &[],
            )
            .expect("failed to query resampled_point_d1");
        rows.iter().map(row_to_resampled_point).collect()
    }

    fn get_by_instrument(&self, instrument: &str) -> Vec<ResampledPointD1> {
        let rows = self
            .conn
            .borrow_mut()
            .query(
                "SELECT id, instrument, open, high, low, close, volume, timestamp, high_low_order \
                 FROM resampled_point_d1 WHERE instrument = $1",
                &[&instrument],
            )
            .expect("failed to query resampled_point_d1 by instrument");
        rows.iter().map(row_to_resampled_point).collect()
    }
}

pub struct PgIndicatorAdapter {
    conn: RefCell<Client>,
}

impl PgIndicatorAdapter {
    pub fn new(connection_string: &str) -> anyhow::Result<Self> {
        let conn = connect(connection_string)?;
        Ok(Self {
            conn: RefCell::new(conn),
        })
    }
}

impl IndicatorRepository for PgIndicatorAdapter {
    fn get_all(&self) -> Vec<Indicator> {
        let rows = self
            .conn
            .borrow_mut()
            .query(
                "SELECT id, type, parameters::text, identifier FROM indicator",
                &[],
            )
            .expect("failed to query indicator");
        rows.iter().map(row_to_indicator).collect()
    }

    fn get_by_type(&self, indicator_type: &str) -> Vec<Indicator> {
        let rows = self
            .conn
            .borrow_mut()
            .query(
                "SELECT id, type, parameters::text, identifier FROM indicator WHERE type = $1",
                &[&indicator_type],
            )
            .expect("failed to query indicator by type");
        rows.iter().map(row_to_indicator).collect()
    }
}
