mod create;
mod create_loop;
mod metadata;

use clap::{value_parser, Arg, ArgMatches, Command};

use crate::config::environment::Environment;

pub enum StrategyCommand {
    Create {
        money_management_strategy_id: i32,
        instrument: String,
    },
    CreateLoop,
    Metadata,
}

pub fn build_strategy_cli() -> Command {
    Command::new("strategy")
        .about("Strategy operations")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create")
                .about("Create a Strategy (fetches data from PostgreSQL)")
                .arg(
                    Arg::new("money_management_strategy_id")
                        .short('m')
                        .long("money_management_strategy_id")
                        .required(true)
                        .value_parser(value_parser!(i32))
                        .help("Money management strategy ID"),
                )
                .arg(
                    Arg::new("instrument")
                        .short('i')
                        .long("instrument")
                        .required(false)
                        .default_value("EURUSD")
                        .help("Instrument pair (default: EURUSD)"),
                ),
        )
        .subcommand(
            Command::new("create_loop")
                .about("Run idempotent strategy pipeline for all (instrument, mms_id) pairs"),
        )
        .subcommand(
            Command::new("metadata")
                .about("Show metadata: DB vs manifest state and remaining triples"),
        )
}

pub fn parse_strategy(m: &ArgMatches) -> StrategyCommand {
    match m.subcommand() {
        Some(("create", sub_m)) => StrategyCommand::Create {
            money_management_strategy_id: *sub_m
                .get_one::<i32>("money_management_strategy_id")
                .unwrap(),
            instrument: sub_m
                .get_one::<String>("instrument")
                .cloned()
                .unwrap_or_else(|| "EURUSD".to_string()),
        },
        Some(("create_loop", _sub_m)) => StrategyCommand::CreateLoop,
        Some(("metadata", _sub_m)) => StrategyCommand::Metadata,
        _ => unreachable!("invalid strategy command"),
    }
}

pub fn resolve_environment() -> Environment {
    std::env::var("ENVIRONMENT")
        .unwrap_or_else(|_| "development".to_string())
        .parse()
        .unwrap_or(Environment::Development)
}

impl StrategyCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            StrategyCommand::Create {
                money_management_strategy_id,
                instrument,
            } => create::execute(money_management_strategy_id, &instrument),
            StrategyCommand::CreateLoop => create_loop::execute(),
            StrategyCommand::Metadata => metadata::execute(),
        }
    }
}
