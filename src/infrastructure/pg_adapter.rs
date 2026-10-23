use std::cell::RefCell;
use std::time::Instant;

use anyhow::{bail, Context};
use log::debug;
use postgres::types::ToSql;
use postgres::{Client, NoTls, Row};
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::{Map, Value};

use crate::domain::indicator::{Indicator, IndicatorParameters, IndicatorRepository};
use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1, ResampledPointD1Repository};

/// Wire format of `indicator.parameters` for `type = "macd"`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MacdParams {
    slow_type: String,
    slow_n: i32,
    fast_type: String,
    fast_n: i32,
}

/// Wire format of `indicator.parameters` for `type = "rsi"`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RsiParams {
    n: i32,
    high_threshold: i32,
    low_threshold: i32,
}

/// Wire format of `indicator.parameters` for the period-only indicator types
/// (`bears`, `bulls`, `dpo`, `force`, `roc`, `vroc`).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PeriodParams {
    period: i32,
}

/// Wire format of `indicator.parameters` for `type = "cho"`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChoParams {
    fast_ma: i32,
    slow_ma: i32,
}

/// Wire format of `indicator.parameters` for `type = "chaikin_volatility"`
/// (type not yet present in the database).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChvParams {
    smooth_period: i32,
    chv_period: i32,
}

/// Wire format of `indicator.parameters` for `type = "osma"`
/// (type not yet present in the database).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OsMaParams {
    fast_ema: i32,
    slow_ema: i32,
    signal_sma: i32,
}

/// Deserializes `indicator.parameters` (fetched as `parameters::text`) into
/// `T`, attaching the indicator type and the raw JSON to any parse error.
fn parse_params<T>(indicator_type: &str, json_text: &str) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str(json_text).with_context(|| {
        format!("invalid parameters for indicator type \"{indicator_type}\": {json_text}")
    })
}

/// Parses `indicator.parameters` for the parameterless indicator types
/// (`ao`, `ac`): the JSON must be an empty object.
fn parse_empty_params(indicator_type: &str, json_text: &str) -> anyhow::Result<()> {
    let map: Map<String, Value> = parse_params(indicator_type, json_text)?;
    if !map.is_empty() {
        let keys = map
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ");
        bail!("indicator type \"{indicator_type}\" takes no parameters, found keys: {keys}");
    }
    Ok(())
}

fn map_high_low_order(value: &str) -> HighLowOrder {
    match value {
        "high_first" => HighLowOrder::HighFirst,
        "low_first" => HighLowOrder::LowFirst,
        _ => HighLowOrder::Undefined,
    }
}

/// Maps a database `indicator.type` string plus its `parameters` JSON onto
/// the domain enum. The matched strings and JSON shapes mirror the database
/// exactly; an unknown type or malformed parameters is an error.
fn map_parameters(indicator_type: &str, json_text: &str) -> anyhow::Result<IndicatorParameters> {
    Ok(match indicator_type {
        "macd" => {
            let p = parse_params::<MacdParams>(indicator_type, json_text)?;
            IndicatorParameters::Macd {
                slow_type: p.slow_type,
                slow_n: p.slow_n,
                fast_type: p.fast_type,
                fast_n: p.fast_n,
            }
        }
        "rsi" => {
            let p = parse_params::<RsiParams>(indicator_type, json_text)?;
            IndicatorParameters::Rsi {
                n: p.n,
                high_threshold: p.high_threshold,
                low_threshold: p.low_threshold,
            }
        }
        "ao" => {
            parse_empty_params(indicator_type, json_text)?;
            IndicatorParameters::AwesomeOscillator
        }
        "ac" => {
            parse_empty_params(indicator_type, json_text)?;
            IndicatorParameters::AcceleratorDecelerator
        }
        "bears" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::BearsPower { period: p.period }
        }
        "bulls" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::BullsPower { period: p.period }
        }
        "cho" => {
            let p = parse_params::<ChoParams>(indicator_type, json_text)?;
            IndicatorParameters::ChaikinOscillator {
                fast_ma: p.fast_ma,
                slow_ma: p.slow_ma,
            }
        }
        "chv" => {
            let p = parse_params::<ChvParams>(indicator_type, json_text)?;
            IndicatorParameters::ChaikinVolatility {
                smooth_period: p.smooth_period,
                chv_period: p.chv_period,
            }
        }
        "dpo" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::DetrendedPriceOscillator { period: p.period }
        }
        "force" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::ForceIndex { period: p.period }
        }
        "osma" => {
            let p = parse_params::<OsMaParams>(indicator_type, json_text)?;
            IndicatorParameters::OsMa {
                fast_ema: p.fast_ema,
                slow_ema: p.slow_ema,
                signal_sma: p.signal_sma,
            }
        }
        "roc" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Roc { period: p.period }
        }
        "trix" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Trix { period: p.period }
        }
        "vroc" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Vroc { period: p.period }
        }
        _ => bail!("unknown indicator type: {indicator_type}"),
    })
}

fn row_to_operation_point(row: &Row) -> OperationPoint {
    OperationPoint {
        id: row.get("id"),
        instrument: row.get("instrument"),
        result: row.get("result"),
        tp: row.get("tp"),
        sl: row.get("sl"),
        risk: row.get("risk"),
        timestamp: i64::from(row.get::<_, i32>("timestamp")),
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
        timestamp: i64::from(row.get::<_, i32>("timestamp")),
        high_low_order: map_high_low_order(&row.get::<_, String>("high_low_order")),
    }
}

fn row_to_indicator(row: &Row) -> anyhow::Result<Indicator> {
    let id: i32 = row.get("id");
    let raw_type: String = row.get("type");
    let raw_params: String = row.get("parameters");
    let parameters = map_parameters(&raw_type, &raw_params)
        .with_context(|| format!("failed to map parameters for indicator id {id}"))?;
    Ok(Indicator { id, parameters })
}

fn connect(connection_string: &str) -> anyhow::Result<Client> {
    Client::connect(connection_string, NoTls).map_err(anyhow::Error::from)
}

fn fetch_rows<T, F>(
    conn: &RefCell<Client>,
    label: &str,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
    map: F,
) -> anyhow::Result<Vec<T>>
where
    F: Fn(&Row) -> anyhow::Result<T>,
{
    let start = Instant::now();
    let rows = conn
        .borrow_mut()
        .query(sql, params)
        .with_context(|| format!("failed to query {label}"))?;
    let duration = start.elapsed();

    debug!(
        "query executed: {label} — {} rows in {duration:?}",
        rows.len()
    );

    rows.iter().map(map).collect()
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

    pub fn get_distinct_mms_ids(&self) -> anyhow::Result<Vec<i32>> {
        fetch_rows(
            &self.conn,
            "long_operation_point distinct mms_ids",
            "SELECT DISTINCT money_management_strategy_id FROM long_operation_point ORDER BY 1",
            &[],
            |r| Ok(r.get("money_management_strategy_id")),
        )
    }
}

impl LongOperationPointRepository for PgLongOperationPointAdapter {
    fn get_all(&self) -> anyhow::Result<Vec<OperationPoint>> {
        fetch_rows(
            &self.conn,
            "long_operation_point get_all",
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM long_operation_point",
            &[],
            |row| Ok(row_to_operation_point(row)),
        )
    }

    fn get_by_mms_id_and_instrument(
        &self,
        mms_id: i32,
        instrument: &str,
    ) -> anyhow::Result<Vec<OperationPoint>> {
        fetch_rows(
            &self.conn,
            "long_operation_point by mms_id and instrument",
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM long_operation_point \
             WHERE money_management_strategy_id = $1 AND instrument = $2",
            &[&mms_id, &instrument],
            |row| Ok(row_to_operation_point(row)),
        )
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
    fn get_all(&self) -> anyhow::Result<Vec<OperationPoint>> {
        fetch_rows(
            &self.conn,
            "short_operation_point get_all",
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM short_operation_point",
            &[],
            |row| Ok(row_to_operation_point(row)),
        )
    }

    fn get_by_mms_id_and_instrument(
        &self,
        mms_id: i32,
        instrument: &str,
    ) -> anyhow::Result<Vec<OperationPoint>> {
        fetch_rows(
            &self.conn,
            "short_operation_point by mms_id and instrument",
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM short_operation_point \
             WHERE money_management_strategy_id = $1 AND instrument = $2",
            &[&mms_id, &instrument],
            |row| Ok(row_to_operation_point(row)),
        )
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

    pub fn get_distinct_instruments(&self) -> anyhow::Result<Vec<String>> {
        fetch_rows(
            &self.conn,
            "resampled_point_d1 distinct instruments",
            "SELECT DISTINCT instrument FROM resampled_point_d1 ORDER BY 1",
            &[],
            |r| Ok(r.get("instrument")),
        )
    }
}

impl ResampledPointD1Repository for PgResampledPointAdapter {
    fn get_all(&self) -> anyhow::Result<Vec<ResampledPointD1>> {
        fetch_rows(
            &self.conn,
            "resampled_point_d1 get_all",
            // ORDER BY timestamp is load-bearing: moving-average indicator math depends on chronological order.
            "SELECT id, instrument, open, high, low, close, volume, timestamp, high_low_order \
             FROM resampled_point_d1 ORDER BY timestamp",
            &[],
            |row| Ok(row_to_resampled_point(row)),
        )
    }

    fn get_by_instrument(&self, instrument: &str) -> anyhow::Result<Vec<ResampledPointD1>> {
        fetch_rows(
            &self.conn,
            "resampled_point_d1 by instrument",
            "SELECT id, instrument, open, high, low, close, volume, timestamp, high_low_order \
             FROM resampled_point_d1 WHERE instrument = $1 ORDER BY timestamp",
            &[&instrument],
            |row| Ok(row_to_resampled_point(row)),
        )
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

    pub fn get_all_indicator_ids(&self) -> anyhow::Result<Vec<i32>> {
        fetch_rows(
            &self.conn,
            "indicator ids",
            "SELECT id FROM indicator ORDER BY 1",
            &[],
            |r| Ok(r.get("id")),
        )
    }
}

impl IndicatorRepository for PgIndicatorAdapter {
    fn get_all(&self) -> anyhow::Result<Vec<Indicator>> {
        fetch_rows(
            &self.conn,
            "indicator get_all",
            "SELECT id, type, parameters::text FROM indicator",
            &[],
            row_to_indicator,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The dev database stores `parameters` as `json` text with one space after
    // each colon; the literals below mirror rows captured from it.

    #[test]
    fn macd_params_parse_from_db_json_shape() {
        let params = map_parameters(
            "macd",
            r#"{"slow_type": "sma", "slow_n": 20, "fast_type": "ema", "fast_n": 21}"#,
        )
        .unwrap();
        assert_eq!(
            params,
            IndicatorParameters::Macd {
                slow_type: "sma".to_string(),
                slow_n: 20,
                fast_type: "ema".to_string(),
                fast_n: 21,
            }
        );
    }

    #[test]
    fn rsi_params_parse_from_db_json_shape() {
        let params = map_parameters(
            "rsi",
            r#"{"n": 10, "high_threshold": 80, "low_threshold": 20}"#,
        )
        .unwrap();
        assert_eq!(
            params,
            IndicatorParameters::Rsi {
                n: 10,
                high_threshold: 80,
                low_threshold: 20,
            }
        );
    }

    #[test]
    fn cho_params_parse_from_db_json_shape() {
        let params = map_parameters("cho", r#"{"fast_ma": 30, "slow_ma": 40}"#).unwrap();
        assert_eq!(
            params,
            IndicatorParameters::ChaikinOscillator {
                fast_ma: 30,
                slow_ma: 40,
            }
        );
    }

    #[test]
    fn types_not_yet_in_db_parse_from_expected_json_shapes() {
        let chv = map_parameters("chv", r#"{"smooth_period": 10, "chv_period": 10}"#).unwrap();
        assert_eq!(
            chv,
            IndicatorParameters::ChaikinVolatility {
                smooth_period: 10,
                chv_period: 10,
            }
        );

        let osma = map_parameters(
            "osma",
            r#"{"fast_ema": 12, "slow_ema": 26, "signal_sma": 9}"#,
        )
        .unwrap();
        assert_eq!(
            osma,
            IndicatorParameters::OsMa {
                fast_ema: 12,
                slow_ema: 26,
                signal_sma: 9,
            }
        );

        let trix = map_parameters("trix", r#"{"period": 14}"#).unwrap();
        assert_eq!(trix, IndicatorParameters::Trix { period: 14 });
    }

    #[test]
    fn period_only_types_parse_from_db_json_shape() {
        let cases = [
            ("bears", IndicatorParameters::BearsPower { period: 105 }),
            ("bulls", IndicatorParameters::BullsPower { period: 105 }),
            (
                "dpo",
                IndicatorParameters::DetrendedPriceOscillator { period: 105 },
            ),
            ("force", IndicatorParameters::ForceIndex { period: 105 }),
            ("roc", IndicatorParameters::Roc { period: 105 }),
            ("vroc", IndicatorParameters::Vroc { period: 105 }),
        ];
        for (indicator_type, expected) in cases {
            assert_eq!(
                map_parameters(indicator_type, r#"{"period": 105}"#).unwrap(),
                expected,
                "type {indicator_type}"
            );
        }
    }

    #[test]
    fn compact_json_also_parses() {
        let params = map_parameters("roc", r#"{"period":105}"#).unwrap();
        assert_eq!(params, IndicatorParameters::Roc { period: 105 });
    }

    #[test]
    fn parameterless_types_parse_from_empty_object() {
        for indicator_type in ["ao", "ac"] {
            assert!(
                map_parameters(indicator_type, "{}").is_ok(),
                "type {indicator_type}"
            );
        }
    }

    #[test]
    fn unknown_type_is_rejected() {
        let err = map_parameters("stochastic", r#"{"n": 1}"#).unwrap_err();
        assert!(err
            .to_string()
            .contains("unknown indicator type: stochastic"));
    }

    #[test]
    fn missing_parameter_key_is_rejected() {
        let err = map_parameters("rsi", r#"{"n": 10, "high_threshold": 80}"#).unwrap_err();
        assert!(format!("{err:#}").contains("missing field"));
    }

    #[test]
    fn string_value_for_integer_parameter_is_rejected() {
        let err = map_parameters("roc", r#"{"period": "105"}"#).unwrap_err();
        assert!(format!("{err:#}").contains("invalid type"));
    }

    #[test]
    fn float_value_for_integer_parameter_is_rejected() {
        assert!(map_parameters("roc", r#"{"period": 10.5}"#).is_err());
    }

    #[test]
    fn unknown_parameter_key_is_rejected() {
        let err = map_parameters("bears", r#"{"period": 5, "signal": 9}"#).unwrap_err();
        assert!(format!("{err:#}").contains("unknown field"));
    }

    #[test]
    fn parameterless_type_with_stray_key_is_rejected() {
        let err = map_parameters("ao", r#"{"n": 14}"#).unwrap_err();
        assert!(err.to_string().contains("takes no parameters"));
    }

    #[test]
    fn non_object_parameters_are_rejected() {
        assert!(map_parameters("vroc", "12").is_err());
    }
}
