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
        .subcommand(
            Command::new("metadata")
                .about("Show metadata: DB vs manifest state and remaining triples"),
        )
        .subcommand(
            Command::new("display")
                .about("Display a strategy with its iteration table")
                .arg(
                    Arg::new("strategy_id")
                        .required(true)
                        .value_parser(value_parser!(i64))
                        .help("Strategy ID to display"),
                )
                .arg(
                    Arg::new("verbose")
                        .short('v')
                        .long("verbose")
                        .required(false)
                        .num_args(0)
                        .help("Show result, tp, sl columns in iteration table"),
                ),
        )
}

pub enum StrategyCommand {
    Create {
        money_management_strategy_id: i32,
        instrument: String,
    },
    Display {
        strategy_id: i64,
        verbose: bool,
    },
    Metadata,
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
        Some(("metadata", _sub_m)) => StrategyCommand::Metadata,
        Some(("display", sub_m)) => StrategyCommand::Display {
            strategy_id: *sub_m.get_one::<i64>("strategy_id").unwrap(),
            verbose: sub_m.get_flag("verbose"),
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
            StrategyCommand::Metadata => {
                super::exec_engine_cli(&["strategy".to_string(), "metadata".to_string()])
            }
            StrategyCommand::Display {
                strategy_id,
                verbose,
            } => {
                let mut args = vec![
                    "strategy".to_string(),
                    "display".to_string(),
                    strategy_id.to_string(),
                ];
                if verbose {
                    args.push("--verbose".to_string());
                }
                super::exec_python_cli(&args)
            }
        }
    }
}
