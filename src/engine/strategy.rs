use clap::{value_parser, Arg, ArgMatches, Command};

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
                money_management_strategy_id,
            } => {
                println!("created strategy mms_id: {money_management_strategy_id}");
                Ok(())
            }
        }
    }
}
