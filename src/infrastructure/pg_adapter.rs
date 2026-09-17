use std::cell::RefCell;
use std::slice::SliceIndex;
use std::time::Instant;

use anyhow::{bail, Context};
use log::debug;
use postgres::types::ToSql;
use postgres::{Client, NoTls, Row};

use crate::domain::indicator::{Indicator, IndicatorParameters, IndicatorRepository};
use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::resampled_point::{HighLowOrder, ResampledPointD1, ResampledPointD1Repository};

/// Safely slices `json` by byte range. The `None` case (a non-UTF-8 boundary)
/// is unreachable for offsets returned by `str::find`, but panic-free slicing
/// requires handling it.
fn get_slice<'a, I>(json: &'a str, index: I, key: &str) -> anyhow::Result<&'a str>
where
    I: SliceIndex<str, Output = str>,
{
    json.get(index).with_context(|| {
        format!("non-UTF-8 boundary while extracting key \"{key}\" from JSON: {json}")
    })
}

fn json_extract_str(json: &str, key: &str) -> anyhow::Result<String> {
    let search = format!("\"{key}\":\"");
    let start = json
        .find(&search)
        .with_context(|| format!("key \"{key}\" not found in JSON: {json}"))?;
    let value_start = start.saturating_add(search.len());
    let rest = get_slice(json, value_start.., key)?;
    let value_end = rest
        .find('"')
        .with_context(|| format!("unterminated string for key \"{key}\" in JSON: {json}"))?;
    Ok(get_slice(rest, ..value_end, key)?.to_string())
}

fn json_extract_i32(json: &str, key: &str) -> anyhow::Result<i32> {
    let search = format!("\"{key}\":");
    let start = json
        .find(&search)
        .with_context(|| format!("key \"{key}\" not found in JSON: {json}"))?;
    let value_start = start.saturating_add(search.len());
    let value_slice = get_slice(json, value_start.., key)?;
    let value_slice = value_slice.trim_start();
    let end = value_slice
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(value_slice.len());
    let raw = get_slice(value_slice, ..end, key)?;
    raw.parse()
        .with_context(|| format!("failed to parse i32 for key \"{key}\" from: {raw}"))
}

fn map_high_low_order(value: &str) -> HighLowOrder {
    match value {
        "high_first" => HighLowOrder::HighFirst,
        "low_first" => HighLowOrder::LowFirst,
        _ => HighLowOrder::Undefined,
    }
}

fn map_parameters(indicator_type: &str, json_text: &str) -> anyhow::Result<IndicatorParameters> {
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
    let raw_type: String = row.get("type");
    let raw_params: String = row.get("parameters");
    Ok(Indicator {
        id: row.get("id"),
        r#type: raw_type.clone(),
        parameters: map_parameters(&raw_type, &raw_params)?,
        identifier: row.get("identifier"),
    })
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
            "SELECT id, instrument, open, high, low, close, volume, timestamp, high_low_order \
             FROM resampled_point_d1",
            &[],
            |row| Ok(row_to_resampled_point(row)),
        )
    }

    fn get_by_instrument(&self, instrument: &str) -> anyhow::Result<Vec<ResampledPointD1>> {
        fetch_rows(
            &self.conn,
            "resampled_point_d1 by instrument",
            "SELECT id, instrument, open, high, low, close, volume, timestamp, high_low_order \
             FROM resampled_point_d1 WHERE instrument = $1",
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
            "SELECT id, type, parameters::text, identifier FROM indicator",
            &[],
            row_to_indicator,
        )
    }

    fn get_by_type(&self, indicator_type: &str) -> anyhow::Result<Vec<Indicator>> {
        fetch_rows(
            &self.conn,
            "indicator by type",
            "SELECT id, type, parameters::text, identifier FROM indicator WHERE type = $1",
            &[&indicator_type],
            row_to_indicator,
        )
    }
}
pub struct PgMoneyManagementStrategyAdapter {
    conn: RefCell<Client>,
}

impl PgMoneyManagementStrategyAdapter {
    pub fn new(connection_string: &str) -> anyhow::Result<Self> {
        let conn = connect(connection_string)?;
        Ok(Self {
            conn: RefCell::new(conn),
        })
    }

    pub fn get_distinct_ids(&self) -> anyhow::Result<Vec<i32>> {
        fetch_rows(
            &self.conn,
            "money_management_strategy distinct ids",
            "SELECT DISTINCT id FROM money_management_strategy ORDER BY 1",
            &[],
            |r| Ok(r.get("id")),
        )
    }
}
