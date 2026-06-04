use clap::{ArgMatches, Command};

pub fn build_strategy_cli() -> Command {
    Command::new("strategy")
        .visible_alias("s")
        .about("Strategy CLI")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("delete").about("Delete Strategies"))
}

pub enum StrategyCommand {
    Delete,
}

pub fn parse_strategy(m: &ArgMatches) -> StrategyCommand {
    match m.subcommand() {
        Some(("delete", _)) => StrategyCommand::Delete,
        _ => unreachable!("invalid strategy command"),
    }
}

impl StrategyCommand {
    pub fn execute(self) -> anyhow::Result<()> {
        match self {
            StrategyCommand::Delete => {
                super::exec_python_cli(&["s".to_string(), "delete".to_string()])
            }
        }
    }
}
