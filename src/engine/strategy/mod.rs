mod create;
mod create_loop;

use clap::{value_parser, Arg, ArgMatches, Command};

pub enum StrategyCommand {
    Create {
        money_management_strategy_id: i32,
        instrument: String,
    },
    CreateLoop,
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
}

pub fn parse_strategy(m: &ArgMatches) -> anyhow::Result<StrategyCommand> {
    match m.subcommand() {
        Some(("create", sub_m)) => Ok(StrategyCommand::Create {
            money_management_strategy_id: *sub_m
                .get_one::<i32>("money_management_strategy_id")
                .ok_or_else(|| {
                    anyhow::anyhow!("missing required argument 'money_management_strategy_id'")
                })?,
            instrument: sub_m
                .get_one::<String>("instrument")
                .cloned()
                .unwrap_or_else(|| "EURUSD".to_string()),
        }),
        Some(("create_loop", _sub_m)) => Ok(StrategyCommand::CreateLoop),
        _ => anyhow::bail!("invalid strategy command"),
    }
}

impl StrategyCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            Self::Create {
                money_management_strategy_id,
                instrument,
            } => create::execute(money_management_strategy_id, &instrument),
            Self::CreateLoop => create_loop::execute(),
        }
    }
}
