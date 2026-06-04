use clap::{value_parser, Arg, ArgMatches, Command};

pub fn build_strategy_cli() -> Command {
    Command::new("strategy")
        .visible_alias("s")
        .about("Strategy CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("delete").about("Delete Strategies"))
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

pub enum StrategyCommand {
    Delete,
    Create { money_management_strategy_id: i32 },
}

pub fn parse_strategy(m: &ArgMatches) -> StrategyCommand {
    match m.subcommand() {
        Some(("delete", _)) => StrategyCommand::Delete,
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
            StrategyCommand::Delete => {
                super::exec_python_cli(&["s".to_string(), "delete".to_string()])
            }
            StrategyCommand::Create {
                money_management_strategy_id,
            } => super::exec_engine_cli(&[
                "strategy".to_string(),
                "create".to_string(),
                "--money_management_strategy_id".to_string(),
                money_management_strategy_id.to_string(),
            ]),
        }
    }
}
