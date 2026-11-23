use std::fmt;
use std::time::{Duration, Instant};

use anyhow::{Context, bail};
use log::debug;
use postgres::fallible_iterator::FallibleIterator;
use postgres::types::{FromSql, ToSql};
use postgres::{Client, Config, NoTls, Row};
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::config::environment::DbConfig;
use crate::domain::indicator::{
    Indicator, IndicatorParameters, IndicatorRepository, MovingAverageType,
};
use crate::domain::operation_point::{
    LongOperationPointRepository, OperationPoint, ShortOperationPointRepository,
};
use crate::domain::resampled_point::{ResampledPointD1, ResampledPointD1Repository};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MacdParams {
    slow_type: String,
    slow_n: i32,
    fast_type: String,
    fast_n: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RsiParams {
    n: i32,
    high_threshold: i32,
    low_threshold: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MfiParams {
    period: i32,
    high_threshold: i32,
    low_threshold: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct StochasticParams {
    k_period: i32,
    slowing: i32,
    high_threshold: i32,
    low_threshold: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WprParams {
    period: i32,
    high_threshold: i32,
    low_threshold: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PeriodParams {
    period: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct VidyaParams {
    cmo_period: i32,
    ema_period: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AmaParams {
    period: i32,
    fast_period: i32,
    slow_period: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChoParams {
    fast_ma: i32,
    slow_ma: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChvParams {
    smooth_period: i32,
    chv_period: i32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OsMaParams {
    fast_ema: i32,
    slow_ema: i32,
    signal_sma: i32,
}

fn parse_params<T>(indicator_type: &str, json_text: &str) -> anyhow::Result<T>
where
    T: DeserializeOwned,
{
    serde_json::from_str(json_text).with_context(|| {
        format!("invalid parameters for indicator type \"{indicator_type}\": {json_text}")
    })
}

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

/// The type strings and JSON shapes mirror the `indicator` table, which is
/// owned by the Django companion.
// One flat arm per indicator type: it grows with every indicator, and
// splitting it would only scatter the mapping.
#[allow(clippy::too_many_lines)]
fn map_parameters(indicator_type: &str, json_text: &str) -> anyhow::Result<IndicatorParameters> {
    Ok(match indicator_type {
        "macd" => {
            let p = parse_params::<MacdParams>(indicator_type, json_text)?;
            IndicatorParameters::Macd {
                slow_type: parse_moving_average_type(&p.slow_type)?,
                slow_n: p.slow_n,
                fast_type: parse_moving_average_type(&p.fast_type)?,
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
        "ama" => {
            let p = parse_params::<AmaParams>(indicator_type, json_text)?;
            IndicatorParameters::Ama {
                period: p.period,
                fast_period: p.fast_period,
                slow_period: p.slow_period,
            }
        }
        "bears" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::BearsPower { period: p.period }
        }
        "bulls" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::BullsPower { period: p.period }
        }
        "cci" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Cci { period: p.period }
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
        "dema" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Dema { period: p.period }
        }
        "dpo" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::DetrendedPriceOscillator { period: p.period }
        }
        "force" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::ForceIndex { period: p.period }
        }
        "frama" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Frama { period: p.period }
        }
        "mfi" => {
            let p = parse_params::<MfiParams>(indicator_type, json_text)?;
            IndicatorParameters::Mfi {
                period: p.period,
                high_threshold: p.high_threshold,
                low_threshold: p.low_threshold,
            }
        }
        "momentum" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Momentum { period: p.period }
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
        "rvi" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Rvi { period: p.period }
        }
        "stochastic" => {
            let p = parse_params::<StochasticParams>(indicator_type, json_text)?;
            IndicatorParameters::Stochastic {
                k_period: p.k_period,
                slowing: p.slowing,
                high_threshold: p.high_threshold,
                low_threshold: p.low_threshold,
            }
        }
        "tema" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Tema { period: p.period }
        }
        "trix" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Trix { period: p.period }
        }
        "vidya" => {
            let p = parse_params::<VidyaParams>(indicator_type, json_text)?;
            IndicatorParameters::Vidya {
                cmo_period: p.cmo_period,
                ema_period: p.ema_period,
            }
        }
        "vroc" => {
            let p = parse_params::<PeriodParams>(indicator_type, json_text)?;
            IndicatorParameters::Vroc { period: p.period }
        }
        "wpr" => {
            let p = parse_params::<WprParams>(indicator_type, json_text)?;
            IndicatorParameters::Wpr {
                period: p.period,
                high_threshold: p.high_threshold,
                low_threshold: p.low_threshold,
            }
        }
        _ => bail!("unknown indicator type: {indicator_type}"),
    })
}

fn parse_moving_average_type(raw: &str) -> anyhow::Result<MovingAverageType> {
    Ok(match raw {
        "ema" => MovingAverageType::Ema,
        "sma" => MovingAverageType::Sma,
        _ => bail!("moving-average type must be \"ema\" or \"sma\", got \"{raw}\""),
    })
}

fn column<'a, T>(row: &'a Row, table: &str, column: &str) -> anyhow::Result<T>
where
    T: FromSql<'a>,
{
    row.try_get(column)
        .with_context(|| format!("failed to read column {table}.{column}"))
}

/// Rejects `sl <= 0`: performance metrics divide by `sl`, so a zero stop
/// loss would yield inf/NaN and silently drop the strategy downstream.
fn row_to_operation_point(row: &Row, table: &str) -> anyhow::Result<OperationPoint> {
    let id: i32 = column(row, table, "id")?;
    let instrument: String = column(row, table, "instrument")?;
    let sl: i32 = column(row, table, "sl")?;
    if sl <= 0 {
        bail!("operation point id {id} ({instrument}) has non-positive sl {sl}");
    }
    Ok(OperationPoint {
        id,
        instrument,
        result: column(row, table, "result")?,
        tp: column(row, table, "tp")?,
        sl,
        risk: column(row, table, "risk")?,
        timestamp: i64::from(column::<i32>(row, table, "timestamp")?),
        money_management_strategy_id: column(row, table, "money_management_strategy_id")?,
    })
}

fn row_to_resampled_point(row: &Row) -> anyhow::Result<ResampledPointD1> {
    const TABLE: &str = "resampled_point_d1";
    Ok(ResampledPointD1 {
        id: column(row, TABLE, "id")?,
        instrument: column(row, TABLE, "instrument")?,
        open: column(row, TABLE, "open")?,
        high: column(row, TABLE, "high")?,
        low: column(row, TABLE, "low")?,
        close: column(row, TABLE, "close")?,
        volume: column(row, TABLE, "volume")?,
        timestamp: i64::from(column::<i32>(row, TABLE, "timestamp")?),
    })
}

fn row_to_indicator(row: &Row) -> anyhow::Result<Indicator> {
    const TABLE: &str = "indicator";
    let id: i32 = column(row, TABLE, "id")?;
    let raw_type: String = column(row, TABLE, "type")?;
    let raw_params: String = column(row, TABLE, "parameters")?;
    let parameters = map_parameters(&raw_type, &raw_params)
        .with_context(|| format!("failed to map parameters for indicator id {id}"))?;
    Ok(Indicator { id, parameters })
}

fn fetch_rows<T, F>(
    conn: &mut Client,
    label: &str,
    sql: &str,
    params: &[&(dyn ToSql + Sync)],
    map: F,
) -> anyhow::Result<Vec<T>>
where
    F: Fn(&Row) -> anyhow::Result<T>,
{
    let start = Instant::now();
    let mut rows = conn
        .query_raw(sql, params.iter().copied())
        .with_context(|| format!("failed to query {label}"))?;
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .with_context(|| format!("failed to query {label}"))?
    {
        out.push(map(&row)?);
    }
    let duration = start.elapsed();

    debug!(
        "query fetched and mapped ({label}): {} rows in {duration:?}",
        out.len()
    );

    Ok(out)
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

/// One PostgreSQL connection serving every repository port.
///
/// The single-cell `create` path needs one of these for its sequential
/// reads; the batch (`create_loop`) path constructs one per table
/// because its loads run concurrently and a sync `postgres::Client`
/// is single-connection.
pub struct PgStore {
    conn: Client,
}

impl fmt::Debug for PgStore {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PgStore").finish_non_exhaustive()
    }
}

impl PgStore {
    pub fn connect(config: &DbConfig) -> anyhow::Result<Self> {
        let conn = Config::new()
            .host(&config.host)
            .port(config.port)
            .user(&config.user)
            .password(&config.password)
            .dbname(&config.database)
            .connect_timeout(CONNECT_TIMEOUT)
            .application_name("bte")
            .connect(NoTls)
            .with_context(|| connection_diagnostics(config))?;
        Ok(Self { conn })
    }
}

fn connection_diagnostics(config: &DbConfig) -> String {
    format!(
        "failed to connect to PostgreSQL\n  \
         Target database: {db}\n  \
         Target host:     {host}:{port}\n  \
         Target user:     {user}\n  \
         Check that:\n  \
         - The database container is running (`docker ps`)\n  \
         - The port is reachable from the engine container \
           (try `psql -h {host} -p {port} -U {user} -d {db}`)\n  \
         - The role \"{user}\" exists in the target database\n  \
         - No local service is occupying port {port} on the host",
        db = config.database,
        host = config.host,
        port = config.port,
        user = config.user,
    )
}

impl LongOperationPointRepository for PgStore {
    fn get_all(&mut self) -> anyhow::Result<Vec<OperationPoint>> {
        fetch_rows(
            &mut self.conn,
            "long_operation_point get_all",
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM long_operation_point \
             ORDER BY instrument, money_management_strategy_id",
            &[],
            |row| row_to_operation_point(row, "long_operation_point"),
        )
    }

    fn get_by_mms_id_and_instrument(
        &mut self,
        mms_id: i32,
        instrument: &str,
    ) -> anyhow::Result<Vec<OperationPoint>> {
        fetch_rows(
            &mut self.conn,
            "long_operation_point by mms_id and instrument",
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM long_operation_point \
             WHERE money_management_strategy_id = $1 AND instrument = $2",
            &[&mms_id, &instrument],
            |row| row_to_operation_point(row, "long_operation_point"),
        )
    }
}

impl ShortOperationPointRepository for PgStore {
    fn get_all(&mut self) -> anyhow::Result<Vec<OperationPoint>> {
        fetch_rows(
            &mut self.conn,
            "short_operation_point get_all",
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM short_operation_point \
             ORDER BY instrument, money_management_strategy_id",
            &[],
            |row| row_to_operation_point(row, "short_operation_point"),
        )
    }

    fn get_by_mms_id_and_instrument(
        &mut self,
        mms_id: i32,
        instrument: &str,
    ) -> anyhow::Result<Vec<OperationPoint>> {
        fetch_rows(
            &mut self.conn,
            "short_operation_point by mms_id and instrument",
            "SELECT id, instrument, result, tp, sl, risk, timestamp, money_management_strategy_id \
             FROM short_operation_point \
             WHERE money_management_strategy_id = $1 AND instrument = $2",
            &[&mms_id, &instrument],
            |row| row_to_operation_point(row, "short_operation_point"),
        )
    }
}

impl ResampledPointD1Repository for PgStore {
    fn get_all(&mut self) -> anyhow::Result<Vec<ResampledPointD1>> {
        fetch_rows(
            &mut self.conn,
            "resampled_point_d1 get_all",
            // ORDER BY timestamp is load-bearing: indicator math assumes
            // chronological order. `high_low_order` exists in the table but
            // nothing reads it.
            "SELECT id, instrument, open, high, low, close, volume, timestamp \
             FROM resampled_point_d1 ORDER BY timestamp",
            &[],
            row_to_resampled_point,
        )
    }

    fn get_by_instrument(&mut self, instrument: &str) -> anyhow::Result<Vec<ResampledPointD1>> {
        fetch_rows(
            &mut self.conn,
            "resampled_point_d1 by instrument",
            "SELECT id, instrument, open, high, low, close, volume, timestamp \
             FROM resampled_point_d1 WHERE instrument = $1 ORDER BY timestamp",
            &[&instrument],
            row_to_resampled_point,
        )
    }
}

impl IndicatorRepository for PgStore {
    fn get_all(&mut self) -> anyhow::Result<Vec<Indicator>> {
        fetch_rows(
            &mut self.conn,
            "indicator get_all",
            "SELECT id, type, parameters::text FROM indicator ORDER BY id",
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
                slow_type: MovingAverageType::Sma,
                slow_n: 20,
                fast_type: MovingAverageType::Ema,
                fast_n: 21,
            }
        );
    }

    #[test]
    fn macd_params_reject_unknown_moving_average_type() {
        let err = map_parameters(
            "macd",
            r#"{"slow_type": "sma", "slow_n": 20, "fast_type": "wma", "fast_n": 21}"#,
        )
        .unwrap_err();
        assert!(format!("{err:#}").contains(r#"got "wma""#), "{err:#}");
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
    fn mfi_params_parse_from_db_json_shape() {
        let params = map_parameters(
            "mfi",
            r#"{"period": 14, "high_threshold": 80, "low_threshold": 20}"#,
        )
        .unwrap();
        assert_eq!(
            params,
            IndicatorParameters::Mfi {
                period: 14,
                high_threshold: 80,
                low_threshold: 20,
            }
        );
    }

    #[test]
    fn stochastic_params_parse_from_db_json_shape() {
        let params = map_parameters(
            "stochastic",
            r#"{"k_period": 5, "slowing": 3, "high_threshold": 80, "low_threshold": 20}"#,
        )
        .unwrap();
        assert_eq!(
            params,
            IndicatorParameters::Stochastic {
                k_period: 5,
                slowing: 3,
                high_threshold: 80,
                low_threshold: 20,
            }
        );
    }

    #[test]
    fn wpr_params_parse_from_db_json_shape() {
        let params = map_parameters(
            "wpr",
            r#"{"period": 14, "high_threshold": -20, "low_threshold": -80}"#,
        )
        .unwrap();
        assert_eq!(
            params,
            IndicatorParameters::Wpr {
                period: 14,
                high_threshold: -20,
                low_threshold: -80,
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

        let dema = map_parameters("dema", r#"{"period": 14}"#).unwrap();
        assert_eq!(dema, IndicatorParameters::Dema { period: 14 });

        let tema = map_parameters("tema", r#"{"period": 14}"#).unwrap();
        assert_eq!(tema, IndicatorParameters::Tema { period: 14 });

        let ama = map_parameters(
            "ama",
            r#"{"period": 10, "fast_period": 2, "slow_period": 30}"#,
        )
        .unwrap();
        assert_eq!(
            ama,
            IndicatorParameters::Ama {
                period: 10,
                fast_period: 2,
                slow_period: 30,
            }
        );

        let vidya = map_parameters("vidya", r#"{"cmo_period": 9, "ema_period": 12}"#).unwrap();
        assert_eq!(
            vidya,
            IndicatorParameters::Vidya {
                cmo_period: 9,
                ema_period: 12,
            }
        );
    }

    #[test]
    fn period_only_types_parse_from_db_json_shape() {
        let cases = [
            ("bears", IndicatorParameters::BearsPower { period: 105 }),
            ("bulls", IndicatorParameters::BullsPower { period: 105 }),
            ("cci", IndicatorParameters::Cci { period: 105 }),
            (
                "dpo",
                IndicatorParameters::DetrendedPriceOscillator { period: 105 },
            ),
            ("force", IndicatorParameters::ForceIndex { period: 105 }),
            ("frama", IndicatorParameters::Frama { period: 105 }),
            ("momentum", IndicatorParameters::Momentum { period: 105 }),
            ("roc", IndicatorParameters::Roc { period: 105 }),
            ("rvi", IndicatorParameters::Rvi { period: 105 }),
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
        let err = map_parameters("ichimoku", r#"{"n": 1}"#).unwrap_err();
        assert!(err.to_string().contains("unknown indicator type: ichimoku"));
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

    fn db_config() -> DbConfig {
        DbConfig {
            host: "db.example".into(),
            port: 6543,
            user: "engine_role".into(),
            password: "s3cret-pw".into(),
            database: "db-staging".into(),
        }
    }

    #[test]
    fn connection_diagnostics_names_target_database_host_port_and_user() {
        let diagnostics = connection_diagnostics(&db_config());

        assert!(diagnostics.contains("Target database: db-staging"));
        assert!(diagnostics.contains("Target host:     db.example:6543"));
        assert!(diagnostics.contains("Target user:     engine_role"));
    }

    #[test]
    fn connection_diagnostics_does_not_leak_password() {
        let diagnostics = connection_diagnostics(&db_config());

        assert!(!diagnostics.contains("s3cret-pw"));
    }
}
