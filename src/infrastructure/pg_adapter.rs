//! PostgreSQL adapters for the domain repository ports.
//!
//! Every operation returns a [`RepositoryError`]-typed result: connection
//! failures, query failures, and row/parameter decoding failures are
//! propagated to the caller, never panicked on. (The only remaining panic
//! source in this module is a `RefCell` double-borrow, which cannot occur:
//! no adapter method reentrantly borrows its connection.)

use std::cell::RefCell;

use postgres::types::{FromSql, ToSql};
use postgres::{Client, NoTls, Row};

use crate::domain::indicator::{Indicator, IndicatorParameters, IndicatorRepository};
use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::repository_error::{RepositoryError, RepositoryResult};
use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1, ResampledPointD1Repository};

// ---------------------------------------------------------------------------
//  Shared helpers
// ---------------------------------------------------------------------------

/// Runs a query, mapping failures to a query error carrying `context`.
fn query_rows(
    conn: &RefCell<Client>,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
    context: &str,
) -> Result<Vec<Row>, RepositoryError> {
    conn.borrow_mut()
        .query(sql, params)
        .map_err(|e| RepositoryError::query(context, e))
}

/// Reads a required column from `row`, mapping failures to a decode error
/// that names the offending column.
fn column<'a, T>(row: &'a Row, name: &str) -> Result<T, RepositoryError>
where
    T: FromSql<'a>,
{
    row.try_get(name)
        .map_err(|e| RepositoryError::decode(format!("decoding column \"{name}\""), e))
}

// ---------------------------------------------------------------------------
//  Indicator parameter JSON
// ---------------------------------------------------------------------------
//
// NOTE: hand-rolled JSON scanning, kept deliberately for now (a follow-up
// replaces it with serde). Known limitations, unchanged from the previous
// version: string values must appear as `"key":"value"` with no whitespace
// around the colon; integer values tolerate whitespace. All failure modes
// return errors instead of panicking.

/// Extracts a string value for `key` from a flat JSON object.
fn json_extract_str(json: &str, key: &str) -> Result<String, RepositoryError> {
    let search = format!("\"{key}\":\"");
    let start = json.find(&search).ok_or_else(|| {
        RepositoryError::invalid_data(format!("string key \"{key}\" not found in JSON: {json}"))
    })?;
    let value_start = start + search.len();
    let value_end = json[value_start..].find('"').ok_or_else(|| {
        RepositoryError::invalid_data(format!(
            "unterminated string for key \"{key}\" in JSON: {json}"
        ))
    })?;
    Ok(json[value_start..value_start + value_end].to_string())
}

/// Extracts an i32 value for `key` from a flat JSON object.
fn json_extract_i32(json: &str, key: &str) -> Result<i32, RepositoryError> {
    let search = format!("\"{key}\":");
    let start = json.find(&search).ok_or_else(|| {
        RepositoryError::invalid_data(format!("key \"{key}\" not found in JSON: {json}"))
    })?;
    let value_start = start + search.len();
    let value_slice = json[value_start..].trim_start();
    let end = value_slice
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(value_slice.len());
    value_slice[..end].parse().map_err(|e| {
        RepositoryError::decode(
            format!(
                "parsing i32 for key \"{key}\" from: {}",
                &value_slice[..end]
            ),
            e,
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

fn map_parameters(
    indicator_type: &str,
    json_text: &str,
) -> Result<IndicatorParameters, RepositoryError> {
    Ok(match indicator_type {
        "macd" => IndicatorParameters::Macd {
            slow_type: json_extract_str(json_text, "slow_type")?,
            slow_n: json_extract_i32(json_text, "slow_n")?,
            fast_type: json_extract_str(json_text, "fast_type")?,
            fast_n: json_extract_i32(json_text, "fast_n")?,
        },
        "rsi" => IndicatorParameters::Rsi {
            n: json_extract_i32(json_text, "n")?,
            high_threshold: json_extract_i32(json_text, "high_threshold")?,
            low_threshold: json_extract_i32(json_text, "low_threshold")?,
        },
        "awesome_oscillator" => IndicatorParameters::AwesomeOscillator,
        "accelerator_decelerator" => IndicatorParameters::AcceleratorDecelerator,
        "bears" => IndicatorParameters::BearsPower {
            period: json_extract_i32(json_text, "period")?,
        },
        "bulls" => IndicatorParameters::BullsPower {
            period: json_extract_i32(json_text, "period")?,
        },
        "chaikin_oscillator" => IndicatorParameters::ChaikinOscillator {
            fast_ma: json_extract_i32(json_text, "fast_ma")?,
            slow_ma: json_extract_i32(json_text, "slow_ma")?,
        },
        "chaikin_volatility" => IndicatorParameters::ChaikinVolatility {
            smooth_period: json_extract_i32(json_text, "smooth_period")?,
            chv_period: json_extract_i32(json_text, "chv_period")?,
        },
        "detrended_price_oscillator" => IndicatorParameters::DetrendedPriceOscillator {
            period: json_extract_i32(json_text, "period")?,
        },
        "force_index" => IndicatorParameters::ForceIndex {
            period: json_extract_i32(json_text, "period")?,
        },
        "osma" => IndicatorParameters::OsMa {
            fast_ema: json_extract_i32(json_text, "fast_ema")?,
            slow_ema: json_extract_i32(json_text, "slow_ema")?,
            signal_sma: json_extract_i32(json_text, "signal_sma")?,
        },
        "roc" => IndicatorParameters::Roc {
            period: json_extract_i32(json_text, "period")?,
        },
        "trix" => IndicatorParameters::Trix {
            period: json_extract_i32(json_text, "period")?,
        },
        "vroc" => IndicatorParameters::Vroc {
            period: json_extract_i32(json_text, "period")?,
        },
        other => {
            return Err(RepositoryError::invalid_data(format!(
                "unknown indicator type: {other}"
            )))
        }
    })
}

// ---------------------------------------------------------------------------
//  Row mapping
// ---------------------------------------------------------------------------

fn row_to_operation_point(row: &Row) -> Result<OperationPoint, RepositoryError> {
    Ok(OperationPoint {
        id: column(row, "id")?,
        instrument: column(row, "instrument")?,
        result: column(row, "result")?,
        tp: column(row, "tp")?,
        sl: column(row, "sl")?,
        risk: column(row, "risk")?,
        timestamp: column::<i32>(row, "timestamp")? as i64,
        money_management_strategy_id: column(row, "money_management_strategy_id")?,
    })
}

fn row_to_resampled_point(row: &Row) -> Result<ResampledPointD1, RepositoryError> {
    let high_low_order: String = column(row, "high_low_order")?;
    Ok(ResampledPointD1 {
        id: column(row, "id")?,
        instrument: column(row, "instrument")?,
        open: column(row, "open")?,
        high: column(row, "high")?,
        low: column(row, "low")?,
        close: column(row, "close")?,
        volume: column(row, "volume")?,
        timestamp: column::<i32>(row, "timestamp")? as i64,
        high_low_order: map_high_low_order(&high_low_order),
    })
}

fn row_to_indicator(row: &Row) -> Result<Indicator, RepositoryError> {
    let raw_type: String = column(row, "type")?;
    let raw_params: String = column(row, "parameters")?;
    let parameters = map_parameters(&raw_type, &raw_params)?;
    Ok(Indicator {
        id: column(row, "id")?,
        r#type: raw_type,
        parameters,
        identifier: column(row, "identifier")?,
    })
}

// ---------------------------------------------------------------------------
//  Connection
// ---------------------------------------------------------------------------

fn connect(connection_string: &str) -> Result<Client, RepositoryError> {
    Client::connect(connection_string, NoTls)
        .map_err(|e| RepositoryError::connection("connecting to PostgreSQL", e))
}

// ---------------------------------------------------------------------------
//  Adapters
// ---------------------------------------------------------------------------

pub struct PgLongOperationPointAdapter {
    conn: RefCell<Client>,
}

impl PgLongOperationPointAdapter {
    pub fn new(connection_string: &str) -> Result<Self, RepositoryError> {
        let conn = connect(connection_string)?;
        Ok(Self {
            conn: RefCell::new(conn),
        })
    }

    pub fn get_distinct_mms_ids(&self) -> RepositoryResult<Vec<i32>> {
        let rows = query_rows(
            &self.conn,
            "SELECT DISTINCT money_management_strategy_id FROM long_operation_point ORDER BY 1",
            &[],
            "querying distinct money_management_strategy_id from long_operation_point",
        )?;
        rows.iter()
            .map(|r| column(r, "money_management_strategy_id"))
            .collect()
    }
}

impl LongOperationPointRepository for PgLongOperationPointAdapter {
    fn get_all(&self) -> RepositoryResult<Vec<OperationPoint>> {
        let rows = query_rows(
            &self.conn,
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM long_operation_point",
            &[],
            "querying long_operation_point",
        )?;
        rows.iter().map(row_to_operation_point).collect()
    }

    fn get_by_mms_id_and_instrument(
        &self,
        mms_id: i32,
        instrument: &str,
    ) -> RepositoryResult<Vec<OperationPoint>> {
        let rows = query_rows(
            &self.conn,
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM long_operation_point \
             WHERE money_management_strategy_id = $1 AND instrument = $2",
            &[&mms_id, &instrument],
            "querying long_operation_point by mms_id and instrument",
        )?;
        rows.iter().map(row_to_operation_point).collect()
    }
}

pub struct PgShortOperationPointAdapter {
    conn: RefCell<Client>,
}

impl PgShortOperationPointAdapter {
    pub fn new(connection_string: &str) -> Result<Self, RepositoryError> {
        let conn = connect(connection_string)?;
        Ok(Self {
            conn: RefCell::new(conn),
        })
    }
}

impl ShortOperationPointRepository for PgShortOperationPointAdapter {
    fn get_all(&self) -> RepositoryResult<Vec<OperationPoint>> {
        let rows = query_rows(
            &self.conn,
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM short_operation_point",
            &[],
            "querying short_operation_point",
        )?;
        rows.iter().map(row_to_operation_point).collect()
    }

    fn get_by_mms_id_and_instrument(
        &self,
        mms_id: i32,
        instrument: &str,
    ) -> RepositoryResult<Vec<OperationPoint>> {
        let rows = query_rows(
            &self.conn,
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM short_operation_point \
             WHERE money_management_strategy_id = $1 AND instrument = $2",
            &[&mms_id, &instrument],
            "querying short_operation_point by mms_id and instrument",
        )?;
        rows.iter().map(row_to_operation_point).collect()
    }
}

pub struct PgResampledPointAdapter {
    conn: RefCell<Client>,
}

impl PgResampledPointAdapter {
    pub fn new(connection_string: &str) -> Result<Self, RepositoryError> {
        let conn = connect(connection_string)?;
        Ok(Self {
            conn: RefCell::new(conn),
        })
    }

    pub fn get_distinct_instruments(&self) -> RepositoryResult<Vec<String>> {
        let rows = query_rows(
            &self.conn,
            "SELECT DISTINCT instrument FROM resampled_point_d1 ORDER BY 1",
            &[],
            "querying distinct instruments from resampled_point_d1",
        )?;
        rows.iter().map(|r| column(r, "instrument")).collect()
    }
}

impl ResampledPointD1Repository for PgResampledPointAdapter {
    fn get_all(&self) -> RepositoryResult<Vec<ResampledPointD1>> {
        let rows = query_rows(
            &self.conn,
            "SELECT id, instrument, open, high, low, close, volume, timestamp, high_low_order \
             FROM resampled_point_d1",
            &[],
            "querying resampled_point_d1",
        )?;
        rows.iter().map(row_to_resampled_point).collect()
    }

    fn get_by_instrument(&self, instrument: &str) -> RepositoryResult<Vec<ResampledPointD1>> {
        let rows = query_rows(
            &self.conn,
            "SELECT id, instrument, open, high, low, close, volume, timestamp, high_low_order \
             FROM resampled_point_d1 WHERE instrument = $1",
            &[&instrument],
            "querying resampled_point_d1 by instrument",
        )?;
        rows.iter().map(row_to_resampled_point).collect()
    }
}

pub struct PgIndicatorAdapter {
    conn: RefCell<Client>,
}

impl PgIndicatorAdapter {
    pub fn new(connection_string: &str) -> Result<Self, RepositoryError> {
        let conn = connect(connection_string)?;
        Ok(Self {
            conn: RefCell::new(conn),
        })
    }

    pub fn get_all_indicator_ids(&self) -> RepositoryResult<Vec<i32>> {
        let rows = query_rows(
            &self.conn,
            "SELECT id FROM indicator ORDER BY 1",
            &[],
            "querying indicator ids",
        )?;
        rows.iter().map(|r| column(r, "id")).collect()
    }
}

impl IndicatorRepository for PgIndicatorAdapter {
    fn get_all(&self) -> RepositoryResult<Vec<Indicator>> {
        let rows = query_rows(
            &self.conn,
            "SELECT id, type, parameters::text, identifier FROM indicator",
            &[],
            "querying indicator",
        )?;
        rows.iter().map(row_to_indicator).collect()
    }

    fn get_by_type(&self, indicator_type: &str) -> RepositoryResult<Vec<Indicator>> {
        let rows = query_rows(
            &self.conn,
            "SELECT id, type, parameters::text, identifier FROM indicator WHERE type = $1",
            &[&indicator_type],
            "querying indicator by type",
        )?;
        rows.iter().map(row_to_indicator).collect()
    }
}

pub struct PgMoneyManagementStrategyAdapter {
    conn: RefCell<Client>,
}

impl PgMoneyManagementStrategyAdapter {
    pub fn new(connection_string: &str) -> Result<Self, RepositoryError> {
        let conn = connect(connection_string)?;
        Ok(Self {
            conn: RefCell::new(conn),
        })
    }

    pub fn get_distinct_ids(&self) -> RepositoryResult<Vec<i32>> {
        let rows = query_rows(
            &self.conn,
            "SELECT DISTINCT id FROM money_management_strategy ORDER BY 1",
            &[],
            "querying distinct money_management_strategy ids",
        )?;
        rows.iter().map(|r| column(r, "id")).collect()
    }
}

// ---------------------------------------------------------------------------
//  Tests (pure functions only — row/query paths need a live database and are
//  covered by integration testing, tracked as a follow-up)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::repository_error::RepositoryErrorKind;
    use std::error::Error as StdError;

    #[test]
    fn map_parameters_parses_every_indicator_type() {
        let cases: Vec<(&str, &str, IndicatorParameters)> = vec![
            (
                "macd",
                r#"{"slow_type":"ema","slow_n":26,"fast_type":"sma","fast_n":12}"#,
                IndicatorParameters::Macd {
                    slow_type: "ema".to_string(),
                    slow_n: 26,
                    fast_type: "sma".to_string(),
                    fast_n: 12,
                },
            ),
            (
                "rsi",
                r#"{"n":14,"high_threshold":70,"low_threshold":30}"#,
                IndicatorParameters::Rsi {
                    n: 14,
                    high_threshold: 70,
                    low_threshold: 30,
                },
            ),
            (
                "awesome_oscillator",
                "{}",
                IndicatorParameters::AwesomeOscillator,
            ),
            (
                "accelerator_decelerator",
                "{}",
                IndicatorParameters::AcceleratorDecelerator,
            ),
            (
                "bears",
                r#"{"period":13}"#,
                IndicatorParameters::BearsPower { period: 13 },
            ),
            (
                "bulls",
                r#"{"period":13}"#,
                IndicatorParameters::BullsPower { period: 13 },
            ),
            (
                "chaikin_oscillator",
                r#"{"fast_ma":3,"slow_ma":10}"#,
                IndicatorParameters::ChaikinOscillator {
                    fast_ma: 3,
                    slow_ma: 10,
                },
            ),
            (
                "chaikin_volatility",
                r#"{"smooth_period":10,"chv_period":10}"#,
                IndicatorParameters::ChaikinVolatility {
                    smooth_period: 10,
                    chv_period: 10,
                },
            ),
            (
                "detrended_price_oscillator",
                r#"{"period":21}"#,
                IndicatorParameters::DetrendedPriceOscillator { period: 21 },
            ),
            (
                "force_index",
                r#"{"period":13}"#,
                IndicatorParameters::ForceIndex { period: 13 },
            ),
            (
                "osma",
                r#"{"fast_ema":12,"slow_ema":26,"signal_sma":9}"#,
                IndicatorParameters::OsMa {
                    fast_ema: 12,
                    slow_ema: 26,
                    signal_sma: 9,
                },
            ),
            (
                "roc",
                r#"{"period":12}"#,
                IndicatorParameters::Roc { period: 12 },
            ),
            (
                "trix",
                r#"{"period":14}"#,
                IndicatorParameters::Trix { period: 14 },
            ),
            (
                "vroc",
                r#"{"period":14}"#,
                IndicatorParameters::Vroc { period: 14 },
            ),
        ];

        for (indicator_type, json, expected) in cases {
            let parsed = map_parameters(indicator_type, json)
                .unwrap_or_else(|e| panic!("failed to parse {indicator_type}: {e}"));
            assert_eq!(parsed, expected, "mismatch for {indicator_type}");
        }
    }

    #[test]
    fn map_parameters_rejects_unknown_type() {
        let err = map_parameters("stochastic", "{}").unwrap_err();
        assert_eq!(err.kind(), RepositoryErrorKind::Decode);
        assert!(err.source().is_none());
        assert!(err
            .to_string()
            .contains("unknown indicator type: stochastic"));
    }

    #[test]
    fn map_parameters_reports_missing_key() {
        let err = map_parameters("rsi", r#"{"n":14}"#).unwrap_err();
        assert_eq!(err.kind(), RepositoryErrorKind::Decode);
        assert!(err.to_string().contains("high_threshold"));
    }

    #[test]
    fn json_extract_i32_parses_negative_and_spaced_values() {
        assert_eq!(json_extract_i32(r#"{"n":-7}"#, "n").unwrap(), -7);
        assert_eq!(json_extract_i32(r#"{"n": 14}"#, "n").unwrap(), 14);
    }

    #[test]
    fn json_extract_i32_rejects_non_numeric_values() {
        let err = json_extract_i32(r#"{"n":"fast"}"#, "n").unwrap_err();
        assert_eq!(err.kind(), RepositoryErrorKind::Decode);
        // The ParseIntError is preserved as the error source.
        assert!(err.source().is_some());
    }

    #[test]
    fn json_extract_str_rejects_unterminated_string() {
        let err = json_extract_str(r#"{"slow_type":"ema"#, "slow_type").unwrap_err();
        assert_eq!(err.kind(), RepositoryErrorKind::Decode);
        assert!(err.to_string().contains("unterminated string"));
    }

    #[test]
    fn map_high_low_order_maps_known_and_unknown_values() {
        assert_eq!(map_high_low_order("high_first"), HighLowOrder::HighFirst);
        assert_eq!(map_high_low_order("low_first"), HighLowOrder::LowFirst);
        assert_eq!(map_high_low_order("nonsense"), HighLowOrder::Undefined);
    }
}
