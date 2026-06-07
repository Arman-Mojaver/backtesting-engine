use clap::{value_parser, Arg, ArgMatches, Command};

pub fn build_strategy_cli() -> Command {
    Command::new("strategy")
        .visible_alias("s")
        .about("Strategy CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(
            Command::new("create")
                .about("Create a Strategy")
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
}

pub enum StrategyCommand {
    Create {
        money_management_strategy_id: i32,
        instrument: String,
    },
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
        _ => unreachable!("invalid strategy command"),
    }
}

impl StrategyCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            StrategyCommand::Create {
                money_management_strategy_id,
                instrument,
            } => super::exec_engine_cli(&[
                "strategy".to_string(),
                "create".to_string(),
                "--money_management_strategy_id".to_string(),
                money_management_strategy_id.to_string(),
                "--instrument".to_string(),
                instrument,
            ]),
        }
    }
}
