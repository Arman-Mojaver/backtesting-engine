use clap::{value_parser, Arg, ArgMatches, Command};

use crate::config::environment::{DbConfig, Environment};
use crate::domain::indicator::IndicatorRepository;
use crate::domain::operation_point::{LongOperationPointRepository, ShortOperationPointRepository};
use crate::domain::resampled_point::ResampledPointD1Repository;
use crate::infrastructure::pg_adapter::{
    PgIndicatorAdapter, PgLongOperationPointAdapter, PgResampledPointAdapter,
    PgShortOperationPointAdapter,
};

pub enum StrategyCommand {
    Create { money_management_strategy_id: i32 },
}

pub fn build_strategy_cli() -> Command {
    Command::new("strategy")
        .about("Strategy operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create").about("Create a Strategy").arg(
                Arg::new("money_management_strategy_id")
                    .short('m')
                    .long("money_management_strategy_id")
                    .required(true)
                    .value_parser(value_parser!(i32))
                    .help("Money management strategy ID"),
            ),
        )
}

pub fn parse_strategy(m: &ArgMatches) -> StrategyCommand {
    match m.subcommand() {
        Some(("create", sub_m)) => StrategyCommand::Create {
            money_management_strategy_id: *sub_m
                .get_one::<i32>("money_management_strategy_id")
                .unwrap(),
        },
        _ => unreachable!("invalid strategy command"),
    }
}

impl StrategyCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            StrategyCommand::Create {
                money_management_strategy_id: mms_id,
            } => {
                let env: Environment = std::env::var("ENVIRONMENT")
                    .unwrap_or_else(|_| "development".to_string())
                    .parse()
                    .unwrap_or(Environment::Development);
                let config = DbConfig::from_env(env);
                let conn_string = config.connection_string();

                eprintln!("Connecting to {} ...", config.database);

                let indicator_adapter = PgIndicatorAdapter::new(&conn_string);
                let long_adapter = PgLongOperationPointAdapter::new(&conn_string);
                let short_adapter = PgShortOperationPointAdapter::new(&conn_string);
                let resampled_adapter = PgResampledPointAdapter::new(&conn_string);

                let indicators = indicator_adapter.get_all();
                println!("=== Indicators ({}) ===", indicators.len());
                for ind in indicators.iter().take(5) {
                    println!(
                        "  #{} type={} id={} params={:?}",
                        ind.id, ind.r#type, ind.identifier, ind.parameters
                    );
                }

                let resampled = resampled_adapter.get_all();
                println!("=== Resampled Points ({}) ===", resampled.len());
                for rp in resampled.iter().take(5) {
                    println!(
                        "  #{} {} O={} H={} L={} C={} V={} ts={} hlo={:?}",
                        rp.id,
                        rp.instrument,
                        rp.open,
                        rp.high,
                        rp.low,
                        rp.close,
                        rp.volume,
                        rp.timestamp,
                        rp.high_low_order
                    );
                }

                let long_ops: Vec<_> = long_adapter
                    .get_all()
                    .into_iter()
                    .filter(|op| op.money_management_strategy_id == mms_id)
                    .collect();
                println!("=== Long Operation Points (mms_id={mms_id}) ===");
                for op in long_ops.iter().take(5) {
                    println!(
                        "  #{} {} result={} tp={} sl={} risk={} mms={} ts={}",
                        op.id,
                        op.instrument,
                        op.result,
                        op.tp,
                        op.sl,
                        op.risk,
                        op.money_management_strategy_id,
                        op.timestamp
                    );
                }

                let short_ops: Vec<_> = short_adapter
                    .get_all()
                    .into_iter()
                    .filter(|op| op.money_management_strategy_id == mms_id)
                    .collect();
                println!("=== Short Operation Points (mms_id={mms_id}) ===");
                for op in short_ops.iter().take(5) {
                    println!(
                        "  #{} {} result={} tp={} sl={} risk={} mms={} ts={}",
                        op.id,
                        op.instrument,
                        op.result,
                        op.tp,
                        op.sl,
                        op.risk,
                        op.money_management_strategy_id,
                        op.timestamp
                    );
                }

                println!();
                println!("Indicators: {}", indicators.len());
                println!("Resampled Points: {}", resampled.len());
                println!("Long Operation Points: {}", long_ops.len());
                println!("Short Operation Points: {}", short_ops.len());

                Ok(())
            }
        }
    }
}
